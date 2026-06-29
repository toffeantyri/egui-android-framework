//! Главный цикл для Decompose-style архитектуры.
//!
//! Событийный цикл — без polling, без фоновых потоков для UI.
//! `UiNotifier` проверяется синхронно в главном потоке.

#![cfg(target_os = "android")]

use std::sync::Arc;
use std::time::{Duration, Instant};

use android_activity::{AndroidApp, MainEvent, PollEvent};

use crate::android::egl_backend::EglState;
use crate::android::input::{self, InputState};
use crate::application::Application;
use crate::ui_notifier::{AndroidWakeHandle, UiNotifier};

use glow::HasContext;

/// Запустить egui-приложение на Android.
pub fn run<A: Application>(app: AndroidApp) {
    let mut app_instance = A::create();

    android_logger::init_once(
        android_logger::Config::default()
            .with_tag(app_instance.config().log_tag.as_str())
            .with_max_level(log::LevelFilter::Info),
    );
    log::info!("run: запуск egui-android-framework");

    app_instance.on_create();
    app_instance.on_start();
    app_instance.on_resume();

    let egui_ctx = egui::Context::default();
    let mut egl_state: Option<EglState> = None;
    let mut egui_painter: Option<egui_glow::Painter> = None;

    let mut last_frame = Instant::now();
    let target_dt = Duration::from_secs_f64(1.0 / app_instance.config().target_fps as f64);

    let mut input_state = InputState::new();
    let mut surface_needs_recreation = false;

    // Создаём waker для Android event loop.
    let waker = app.create_waker();
    let wake_handle = AndroidWakeHandle::new(move || {
        waker.wake();
    });
    log::info!("run: Android waker создан");

    // Реактивный уведомитель — создаётся Application после EGL init.
    let mut notifier: Option<UiNotifier> = None;
    let mut notifier_initialized = false;

    loop {
        // --- Слив системных событий ---
        let mut destroy_requested = false;
        app.poll_events(Some(Duration::from_millis(16)), |event| match event {
            PollEvent::Wake | PollEvent::Timeout => {
                // Wake от data layer или таймаут — идём в рендеринг
            }
            PollEvent::Main(e) => match e {
                MainEvent::InitWindow { .. } => {
                    log::info!("Lifecycle: InitWindow");
                    surface_needs_recreation = true;
                }
                MainEvent::Resume { .. } => {
                    log::info!("Lifecycle: Resume");
                    app_instance.on_resume();
                }
                MainEvent::Pause { .. } => {
                    log::info!("Lifecycle: Pause");
                    app_instance.on_pause();
                }
                MainEvent::Stop { .. } => {
                    log::info!("Lifecycle: Stop");
                    app_instance.on_stop();
                }
                MainEvent::Destroy { .. } => {
                    log::info!("Lifecycle: Destroy");
                    destroy_requested = true;
                }
                _ => {}
            },
            _ => {}
        });

        // --- Ввод: сливаем ВСЕ накопившиеся события ---
        {
            let pp = egui_ctx.pixels_per_point();
            input::process_input_events(&app, pp, &mut input_state);
        }

        if input_state.back_pressed {
            log::info!("Back нажата — выход");
            destroy_requested = true;
        }

        if destroy_requested {
            app_instance.on_destroy();
            if let Some(ref mut p) = egui_painter {
                p.destroy();
            }
            egui_painter = None;
            if let Some(ref mut state) = egl_state {
                state.destroy();
            }
            egl_state = None;
            log::info!("Выход из главного цикла (run)");
            break;
        }

        // --- Пересоздание EGL surface ---
        if surface_needs_recreation {
            if let Some(nw) = app.native_window() {
                if let Some(ref mut state) = egl_state {
                    match state.recreate_surface(&nw) {
                        Ok(()) => {
                            if let Some(ref mut painter) = egui_painter {
                                unsafe {
                                    gl_clear(painter.gl());
                                }
                            }
                            let pp = compute_pixels_per_point(nw.width(), nw.height());
                            egui_ctx.set_pixels_per_point(pp);
                        }
                        Err(e) => {
                            log::error!("Не удалось пересоздать EGL surface: {}", e);
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

        // --- Инициализация EGL ---
        if egl_state.is_none() {
            if let Some(nw) = app.native_window() {
                match EglState::create(&nw) {
                    Ok(state) => {
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
                                unsafe {
                                    gl_clear(painter.gl());
                                }
                                egui_ctx.set_fonts(egui::FontDefinitions::default());
                                let pp = compute_pixels_per_point(nw.width(), nw.height());
                                egui_ctx.set_pixels_per_point(pp);
                                egl_state = Some(state);
                                egui_painter = Some(painter);

                                if !notifier_initialized {
                                    notifier_initialized = true;
                                    log::info!("run: инициализируем UiNotifier");
                                    notifier = app_instance
                                        .create_notifier(&egui_ctx, wake_handle.clone());
                                }
                            }
                            Err(e) => {
                                log::error!("Painter::new не удался: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("EglState::create не удался: {}", e);
                    }
                }
            }
        }

        // --- Проверка уведомлений от data layer ---
        if let Some(ref mut n) = notifier {
            n.check();
        }

        // --- Рендеринг ---
        let now = Instant::now();
        if now.duration_since(last_frame) >= target_dt {
            last_frame = now;

            if let (Some(ref mut painter), Some(ref state)) = (&mut egui_painter, &egl_state) {
                let (w, h) = app
                    .native_window()
                    .map(|nw| (nw.width() as u32, nw.height() as u32))
                    .unwrap_or((0, 0));
                if w == 0 || h == 0 {
                    continue;
                }

                let pp = egui_ctx.pixels_per_point();
                let top_inset = 45.0_f32.min(h as f32 / pp * 0.06);
                let bottom_inset = 48.0_f32.min(h as f32 / pp * 0.06);

                // Забираем ВСЕ накопившиеся события одним блоком —
                // DOWN и UP, пришедшие между кадрами, попадают в один run().
                let events = std::mem::take(&mut input_state.events);

                let raw_input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::new(0.0, top_inset),
                        egui::vec2(w as f32 / pp, (h as f32 / pp) - top_inset - bottom_inset),
                    )),
                    events,
                    ..egui::RawInput::default()
                };

                let full_output = app_instance.frame(&egui_ctx, raw_input);

                unsafe {
                    let gl = painter.gl();
                    gl.clear_color(0.2, 0.2, 0.2, 1.0);
                    gl.clear(glow::COLOR_BUFFER_BIT);
                }

                let primitives =
                    egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
                painter.paint_and_update_textures(
                    [w, h],
                    full_output.pixels_per_point,
                    &primitives,
                    &full_output.textures_delta,
                );

                if let Err(e) = state.swap_buffers() {
                    log::warn!("swap_buffers: {}", e);
                    if let Some(ref mut p) = egui_painter {
                        p.destroy();
                    }
                    egui_painter = None;
                    egl_state = None;
                }
            }
        }
    }
}

fn compute_pixels_per_point(width: i32, height: i32) -> f32 {
    let w = width.max(height) as f32;
    (w / 400.0).max(1.0).min(5.0)
}

unsafe fn gl_clear(gl: &glow::Context) {
    gl.clear_color(0.0, 0.0, 0.0, 0.0);
    gl.clear(glow::COLOR_BUFFER_BIT);
}

fn get_proc_address(name: &std::ffi::CStr) -> *const std::os::raw::c_void {
    type EglGetProcAddress =
        unsafe extern "system" fn(*const std::os::raw::c_char) -> *const std::os::raw::c_void;

    let handle = unsafe {
        libc::dlopen(
            "libEGL.so\0".as_ptr() as *const std::os::raw::c_char,
            libc::RTLD_LAZY | libc::RTLD_NOLOAD,
        )
    };
    if handle.is_null() {
        log::error!("get_proc_address: libEGL.so не загружен");
        return std::ptr::null();
    }

    let sym = unsafe {
        libc::dlsym(
            handle,
            "eglGetProcAddress\0".as_ptr() as *const std::os::raw::c_char,
        )
    };
    if sym.is_null() {
        log::error!("get_proc_address: eglGetProcAddress не найден");
        return std::ptr::null();
    }

    let func: EglGetProcAddress = unsafe { std::mem::transmute(sym) };
    let addr = unsafe { func(name.as_ptr()) };

    if addr.is_null() {
        let handle2 = unsafe {
            libc::dlopen(
                "libGLESv2.so\0".as_ptr() as *const std::os::raw::c_char,
                libc::RTLD_LAZY | libc::RTLD_NOLOAD,
            )
        };
        if let Ok(s) = name.to_str() {
            log::warn!("eglGetProcAddress вернул null для {}", s);
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
