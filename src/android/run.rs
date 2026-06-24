//! Главный цикл приложения для Android: обработка жизненного цикла,
//! touch-ввод, инициализация EGL/egui и рендеринг.
//!
//! Точка входа — [`run()`], которая принимает конкретный тип,
//! реализующий [`Application`].

#![cfg(target_os = "android")]

use std::sync::Arc;
use std::time::{Duration, Instant};

use android_activity::{AndroidApp, MainEvent, PollEvent};

use crate::activity::Activity;
use crate::android::egl_backend::EglState;
use crate::android::input::{self, InputState};
use crate::{AppContext, Application, LifecycleObserver, ViewModel};

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
            .with_tag(app_ctx.config().log_tag.as_str())
            .with_max_level(log::LevelFilter::Info),
    );
    log::info!("run: starting egui-android-framework app");

    // create_view_model вызывается первой — она сама создаёт каналы
    // через ctx.view_model_context(). Затем run() получает свою копию vm_ctx
    // с тем же Receiver (через Arc), чтобы опрашивать события из data layer.
    let mut view_model = A::create_view_model(&mut app_ctx);
    let vm_ctx = app_ctx.view_model_context();
    let mut activity = A::create_activity(&mut app_ctx);

    activity.on_create();
    activity.on_start();

    let egui_ctx = egui::Context::default();
    let mut egl_state: Option<EglState> = None;
    let mut egui_painter: Option<egui_glow::Painter> = None;

    let mut needs_redraw = true;
    let mut last_frame = Instant::now();
    let target_dt = Duration::from_secs_f64(1.0 / app_ctx.config().target_fps as f64);

    // Состояние ввода (touch, клавиатура)
    let mut input_state = InputState::new();

    // Флаг: EGL display существует, но surface устарел из-за InitWindow.
    // Устанавливается в колбэке InitWindow, сбрасывается после пересоздания surface.
    let mut surface_needs_recreation = false;

    loop {
        input_state.clear();

        // --- Обработка событий жизненного цикла ---
        let mut destroy_requested = false;
        app.poll_events(None, |event| match event {
            PollEvent::Wake | PollEvent::Timeout => {}
            PollEvent::Main(e) => match e {
                MainEvent::InitWindow { .. } => {
                    // Получено новое окно (например, после Resume).
                    // Не разрушаем EGL display/context — только surface будет пересоздан
                    // после poll_events, где у нас есть доступ к egl_state.
                    log::info!("Lifecycle: InitWindow — scheduling surface recreation");
                    surface_needs_recreation = true;
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
                    // Не разрушаем EGL — activity может вернуться через
                    // Resume → InitWindow с тем же display/context.
                    log::info!("Lifecycle: Stop — preserving EGL state");
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

        // --- Пересоздание EGL surface при смене окна (InitWindow) ---
        if surface_needs_recreation {
            if let Some(nw) = app.native_window() {
                if let Some(ref mut state) = egl_state {
                    log::info!(
                        "Recreating EGL surface for new window {}x{}",
                        nw.width(),
                        nw.height()
                    );
                    match state.recreate_surface(&nw) {
                        Ok(()) => {
                            // Не разрушаем painter — переиспользуем его.
                            // Сбрасываем GL состояние после make_current с новым surface.
                            if let Some(ref mut painter) = egui_painter {
                                unsafe {
                                    gl_clear(painter.gl());
                                }
                            }
                            log::info!("EGL surface recreated successfully");
                        }
                        Err(e) => {
                            log::error!("Failed to recreate EGL surface: {}", e);
                            // Если не удалось пересоздать surface, разрушаем всё —
                            // будет инициализировано заново при следующем native_window().
                            if let Some(ref mut p) = egui_painter {
                                p.destroy();
                            }
                            egui_painter = None;
                            if let Some(ref mut s) = egl_state {
                                s.destroy();
                            }
                            egl_state = None;
                        }
                    }
                }
            }
            surface_needs_recreation = false;
        }

        // --- Инициализация EGL с нуля (первый запуск или после полной очистки) ---
        if egl_state.is_none() {
            if let Some(nw) = app.native_window() {
                log::info!(
                    "Initializing EGL + egui... size={}x{}",
                    nw.width(),
                    nw.height()
                );
                match EglState::create(&nw) {
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
