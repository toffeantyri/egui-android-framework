//! Главный цикл приложения для Android: обработка жизненного цикла,
//! touch-ввод, инициализация EGL/egui и рендеринг.
//!
//! Точка входа — [`run()`], которая принимает конкретный тип,
//! реализующий [`Application`].

#![cfg(target_os = "android")]

use std::sync::Arc;
use std::time::{Duration, Instant};

use android_activity::{AndroidApp, MainEvent, PollEvent};
use ndk::native_window::NativeWindow;

use crate::android::egl_backend::EglState;
use crate::android::input::{self, InputState};
use crate::{AppContext, Application, LifecycleObserver, ViewModel, ViewModelContext};

use glow::HasContext;

/// Запустить egui-приложение на Android.
///
/// Вызывается из `#[no_mangle] pub fn android_main(app: AndroidApp)` в крейте пользователя,
/// которая настраивает конкретный тип приложения.
pub fn run<A: Application>(app: AndroidApp) {
    let mut app_ctx = AppContext::<A>::new();
    A::on_create(&mut app_ctx);

    // Инициализируем логгер с тегом из конфига приложения
    android_logger::init_once(
        android_logger::Config::default()
            .with_tag(&app_ctx.config().log_tag)
            .with_max_level(log::LevelFilter::Info),
    );
    log::info!("run: starting egui-android-framework app");

    let vm_ctx = app_ctx.view_model_context();
    let mut view_model = A::create_view_model(&mut app_ctx);
    let mut activity = A::create_activity(&mut app_ctx);

    activity.on_create();
    activity.on_start();

    let egui_ctx = egui::Context::default();
    let mut egl_state: Option<EglState> = None;
    let mut egui_painter: Option<egui_glow::Painter> = None;

    // Храним указатель на текущее native window (ANativeWindow*),
    // чтобы при смене окна (конфигурация/пересоздание) пересоздать EGL.
    let mut current_native_window: Option<NativeWindow> = None;

    let mut needs_redraw = true;
    let mut last_frame = Instant::now();
    let target_dt = Duration::from_secs_f64(1.0 / app_ctx.config().target_fps as f64);

    // Состояние ввода (touch, клавиатура)
    let mut input_state = InputState::new();

    loop {
        input_state.clear();

        // --- Обработка событий жизненного цикла ---
        let mut destroy_requested = false;
        app.poll_events(None, |event| match event {
            PollEvent::Wake | PollEvent::Timeout => {}
            PollEvent::Main(e) => match e {
                MainEvent::InitWindow { .. } => {
                    log::info!("Lifecycle: InitWindow");
                    needs_redraw = true;
                }
                MainEvent::Resume { .. } => {
                    log::info!("Lifecycle: Resume");
                    activity.on_resume();
                    needs_redraw = true;
                }
                MainEvent::Pause { .. } => {
                    log::info!("Lifecycle: Pause");
                    activity.on_pause();
                }
                MainEvent::Stop { .. } => {
                    log::info!("Lifecycle: Stop");
                    activity.on_stop();
                }
                MainEvent::Destroy { .. } => {
                    log::info!("Lifecycle: Destroy — exiting");
                    destroy_requested = true;
                }
                MainEvent::RedrawNeeded { .. } => {
                    needs_redraw = true;
                }
                _ => {}
            },
            _ => {}
        });

        if destroy_requested {
            activity.on_destroy();
            if let Some(ref mut p) = egui_painter {
                p.destroy();
            }
            egui_painter = None;
            if let Some(ref mut state) = egl_state {
                state.destroy();
            }
            egl_state = None;
            std::process::exit(0);
        }

        // --- Ввод (touch, клавиатура, back) ---
        let pp = egui_ctx.pixels_per_point();
        input::process_input_events(&app, pp, &mut input_state);

        // --- Обработка кнопки Back ---
        if input_state.back_pressed {
            if activity.on_back_pressed(&mut view_model) {
                log::info!("Back pressed — handled by activity");
            } else {
                log::info!("Back pressed — not handled, ignoring");
            }
        }

        // --- Инициализация/пересоздание EGL при получении окна ---
        if let Some(nw) = app.native_window() {
            // Проверяем, сменилось ли окно (по сырому указателю ANativeWindow*)
            let window_changed = current_native_window
                .as_ref()
                .map(|old| old.raw_window_handle() != nw.raw_window_handle())
                .unwrap_or(true);

            if window_changed {
                // Сброс EGL — будет пересоздан ниже
                if egl_state.is_some() {
                    log::info!("Window changed, resetting EGL");
                    if let Some(ref mut p) = egui_painter {
                        p.destroy();
                    }
                    egui_painter = None;
                    if let Some(ref mut state) = egl_state {
                        state.destroy();
                    }
                    egl_state = None;
                }
                current_native_window = Some(nw);
            }

            // Захватываем nw снова (после того как могли переместить его выше)
            let nw_ref = current_native_window.as_ref().unwrap();

            if egl_state.is_none() {
                log::info!(
                    "Initializing EGL + egui... size={}x{}",
                    nw_ref.width(),
                    nw_ref.height()
                );
                match EglState::create(nw_ref) {
                    Ok(state) => {
                        // Создаём glow контекст через загрузчик из EGL
                        let gl = unsafe {
                            glow::Context::from_loader_function(|name| {
                                let cname = std::ffi::CStr::from_ptr(
                                    name.as_ptr() as *const std::os::raw::c_char
                                );
                                get_proc_address(cname)
                            })
                        };

                        match egui_glow::Painter::new(Arc::new(gl), "", None, true) {
                            Ok(painter) => {
                                // Сбрасываем состояние GL
                                unsafe {
                                    gl_clear(painter.gl());
                                }
                                // Перезагружаем шрифты
                                egui_ctx.set_fonts(egui::FontDefinitions::default());

                                egl_state = Some(state);
                                egui_painter = Some(painter);
                                log::info!("EGL + egui painter initialized!");
                                needs_redraw = true;
                            }
                            Err(e) => {
                                log::error!("egui_glow::Painter::new failed: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("EglState::create failed: {}", e);
                    }
                }
            }
        }

        // --- События из data layer (ViewModelContext::poll_events) ---
        let events = vm_ctx.poll_events();
        if !events.is_empty() {
            for evt in events {
                view_model.on_event(evt);
            }
            needs_redraw = true;
        }

        // --- Рендеринг ---
        let now = Instant::now();
        if needs_redraw && now.duration_since(last_frame) >= target_dt {
            needs_redraw = false;
            last_frame = now;

            if let (Some(ref mut painter), Some(ref state)) = (&mut egui_painter, &egl_state) {
                let (w, h) = app
                    .native_window()
                    .map(|nw| (nw.width() as u32, nw.height() as u32))
                    .unwrap_or((0, 0));
                if w == 0 || h == 0 {
                    // Surface ещё не готов
                    continue;
                }

                let pp = egui_ctx.pixels_per_point();
                let top_inset = 45.0_f32.min(h as f32 / pp * 0.06);
                let bottom_inset = 48.0_f32.min(h as f32 / pp * 0.06);

                let raw_input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::new(0.0, top_inset),
                        egui::vec2(w as f32 / pp, (h as f32 / pp) - top_inset - bottom_inset),
                    )),
                    events: input_state.events.clone(),
                    ..egui::RawInput::default()
                };

                let full_output = egui_ctx.run(raw_input, |ctx| {
                    activity.render(ctx, &view_model);
                });

                // Очищаем экран тёмно-серым
                unsafe {
                    let gl = painter.gl();
                    gl.clear_color(0.2, 0.2, 0.2, 1.0);
                    gl.clear(glow::COLOR_BUFFER_BIT);
                }

                let primitives: Vec<egui::epaint::ClippedPrimitive> =
                    egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
                painter.paint_and_update_textures(
                    [w, h],
                    full_output.pixels_per_point,
                    &primitives,
                    &full_output.textures_delta,
                );

                if let Err(e) = state.swap_buffers() {
                    log::warn!("swap_buffers error: {}", e);
                    // При ошибке swap нужно пересоздать EGL
                    if let Some(ref mut p) = egui_painter {
                        p.destroy();
                    }
                    egui_painter = None;
                    egl_state = None;
                }
            }
        }

        // Небольшая задержка для снижения нагрузки на CPU
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// Очистить GL буфер (чёрный прозрачный).
unsafe fn gl_clear(gl: &glow::Context) {
    gl.clear_color(0.0, 0.0, 0.0, 0.0);
    gl.clear(glow::COLOR_BUFFER_BIT);
}

/// Получить адрес OpenGL функции через eglGetProcAddress.
///
/// Использует `dlopen`/`dlsym` для вызова `eglGetProcAddress` из libEGL.so.
/// Fallback — прямой `dlsym` на libGLESv2.so.
fn get_proc_address(name: &std::ffi::CStr) -> *const std::os::raw::c_void {
    type EglGetProcAddress =
        unsafe extern "system" fn(*const std::os::raw::c_char) -> *const std::os::raw::c_void;

    // libEGL.so уже загружена процессом android-activity
    let handle = unsafe {
        libc::dlopen(
            "libEGL.so\0".as_ptr() as *const std::os::raw::c_char,
            libc::RTLD_LAZY | libc::RTLD_NOLOAD,
        )
    };
    if handle.is_null() {
        log::error!("get_proc_address: libEGL.so not loaded");
        return std::ptr::null();
    }

    let sym = unsafe {
        libc::dlsym(
            handle,
            "eglGetProcAddress\0".as_ptr() as *const std::os::raw::c_char,
        )
    };
    if sym.is_null() {
        log::error!("get_proc_address: eglGetProcAddress not found");
        return std::ptr::null();
    }

    let func: EglGetProcAddress = unsafe { std::mem::transmute(sym) };
    let addr = unsafe { func(name.as_ptr()) };

    if addr.is_null() {
        // fallback: dlsym на саму GL функцию из libGLESv2.so
        let handle2 = unsafe {
            libc::dlopen(
                "libGLESv2.so\0".as_ptr() as *const std::os::raw::c_char,
                libc::RTLD_LAZY | libc::RTLD_NOLOAD,
            )
        };
        if let Ok(s) = name.to_str() {
            log::warn!("eglGetProcAddress returned null for {}", s);
        }
        if !handle2.is_null() {
            let fallback = unsafe { libc::dlsym(handle2, name.as_ptr()) };
            if !fallback.is_null() {
                return fallback;
            }
        }
    }
    addr
}
