//! Graphics pipeline — создание Painter, рендеринг, scissor test.
//!
//! Вынесен из `run.rs` для разделения ответственности.
//!
//! Содержит:
//! - [`GraphicsPipeline`] — владеет `egui_glow::Painter` и `glow::Context`
//! - Инициализация Painter через `from_loader_function` + dlsym
//! - Рендеринг кадра: scissor test, clear, tessellate, paint, swap buffers

#![cfg(target_os = "android")]

use std::sync::Arc;

use glow::HasContext;

use crate::backend::AndroidBackend;
use crate::event::Insets;
use egui_android_platform::Waker;
use egui_android_runtime::{Application, RuntimeContext};

/// Graphics pipeline — владеет Painter и контекстом OpenGL.
///
/// # Жизненный цикл
///
/// 1. Создаётся после первого InitWindow (когда есть EGL surface).
/// 2. Живёт всё время жизни приложения.
/// 3. Уничтожается при Destroy.
///
/// # Безопасность
///
/// Весь unsafe-код (glow::HasContext) изолирован в этом модуле.
pub struct GraphicsPipeline {
    pub painter: egui_glow::Painter,
    gl: Arc<glow::Context>,
}

impl GraphicsPipeline {
    /// Попытаться создать GraphicsPipeline.
    ///
    /// 1. Загружает OpenGL функции через `dlsym` + `eglGetProcAddress`.
    /// 2. Создаёт `egui_glow::painter::Context` через `from_loader_function`.
    /// 3. Создаёт `egui_glow::Painter`.
    /// 4. Инициализирует `RuntimeContext` (если ещё не инициализирован).
    ///
    /// Возвращает `None`, если создание не удалось.
    pub fn try_new<A: Application>(
        backend: &mut dyn AndroidBackend,
        app_instance: &mut A,
        egui_ctx: &egui::Context,
        waker: &Waker,
        rt_ctx: &mut Option<RuntimeContext>,
        rt_ctx_initialized: &mut bool,
    ) -> Option<Self> {
        // Проверяем, инициализирована ли графика через surface_handle
        let has_graphics = !backend.surface_handle().as_egl_surface().is_null();
        if !has_graphics {
            return None;
        }

        let gl = unsafe {
            egui_glow::painter::Context::from_loader_function(|name| {
                let cname = std::ffi::CStr::from_ptr(name.as_ptr() as *const std::os::raw::c_char);
                get_proc_address(cname)
            })
        };

        match egui_glow::Painter::new(Arc::new(gl), "", None, true) {
            Ok(painter) => {
                unsafe {
                    let gl_ptr = &*painter.gl();
                    gl_ptr.clear_color(0.0, 0.0, 0.0, 1.0);
                    gl_ptr.clear(glow::COLOR_BUFFER_BIT);
                }

                if !*rt_ctx_initialized {
                    *rt_ctx_initialized = true;
                    log::info!("graphics: инициализируем RuntimeContext");
                    *rt_ctx = Some(app_instance.create_runtime_context(egui_ctx, waker.clone()));
                }

                // Сохраняем Arc<glow::Context> для доступа из render_frame
                let gl = Arc::clone(painter.gl());

                Some(Self { painter, gl })
            }
            Err(e) => {
                log::error!("Painter::new не удался: {:?}", e);
                None
            }
        }
    }

    /// Отрендерить один кадр.
    ///
    /// 1. Выключает scissor test, очищает буфер цветом фона.
    /// 2. Включает scissor test с учётом insets (чтобы контент не рисовался под системными барами).
    /// 3. Tessellates shapes из `full_output`.
    /// 4. Рисует через `painter.paint_and_update_textures`.
    /// 5. Swap buffers через backend.
    ///
    /// Возвращает `true`, если swap был успешен, `false` если painter нужно пересоздать.
    pub fn render_frame(
        &mut self,
        egui_ctx: &egui::Context,
        full_output: &egui::FullOutput,
        window_size: (u32, u32),
        clear_color: (f32, f32, f32),
        insets_pts: (f32, f32, f32, f32),
        pixels_per_point: f32,
        backend: &mut dyn AndroidBackend,
    ) -> bool {
        let (w, h) = window_size;
        let (inset_l, inset_t, inset_r, inset_b) = insets_pts;

        unsafe {
            let gl_ptr = &*self.painter.gl();

            gl_ptr.disable(glow::SCISSOR_TEST);
            gl_ptr.clear_color(clear_color.0, clear_color.1, clear_color.2, 1.0);
            gl_ptr.clear(glow::COLOR_BUFFER_BIT);

            let scissor_x = (inset_l * pixels_per_point) as i32;
            let scissor_y = (inset_b * pixels_per_point) as i32;
            let scissor_w =
                (w as f32 - inset_l * pixels_per_point - inset_r * pixels_per_point) as i32;
            let scissor_h =
                (h as f32 - inset_t * pixels_per_point - inset_b * pixels_per_point) as i32;
            gl_ptr.enable(glow::SCISSOR_TEST);
            gl_ptr.scissor(scissor_x, scissor_y, scissor_w.max(1), scissor_h.max(1));
        }

        let primitives =
            egui_ctx.tessellate(full_output.shapes.clone(), full_output.pixels_per_point);

        self.painter.paint_and_update_textures(
            [w, h],
            full_output.pixels_per_point,
            &primitives,
            &full_output.textures_delta,
        );

        // Swap buffers через backend
        if let Err(e) = backend.swap_buffers() {
            log::warn!("swap_buffers: {}", e);
            return false;
        }

        true
    }

    /// Уничтожить painter.
    pub fn destroy(&mut self) {
        self.painter.destroy();
    }
}

/// Получить адрес OpenGL функции через dlsym.
///
/// Сначала пробует `dlsym(RTLD_DEFAULT)`.
/// Если не найден — fallback на `eglGetProcAddress`.
fn get_proc_address(name: &std::ffi::CStr) -> *const std::ffi::c_void {
    unsafe {
        let func = libc::dlsym(libc::RTLD_DEFAULT, name.as_ptr());
        if !func.is_null() {
            return func;
        }
    }
    // Fallback на eglGetProcAddress
    unsafe {
        type EglGetProcAddress =
            unsafe extern "system" fn(*const libc::c_char) -> *const std::ffi::c_void;
        let egl_get_proc_addr: EglGetProcAddress = std::mem::transmute(libc::dlsym(
            libc::RTLD_DEFAULT,
            b"eglGetProcAddress\0".as_ptr() as *const libc::c_char,
        ));
        egl_get_proc_addr(name.as_ptr())
    }
}
