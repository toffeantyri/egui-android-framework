//! Главный цикл для Decompose-style архитектуры.
//!
//! Событийный цикл — без polling, без фоновых потоков для UI.
//! `UiNotifier` проверяется синхронно в главном потоке.
//!
//! # Архитектура потока данных
//!
//! BackendEvent (от GlBackend/NativeBackend)
//!       ↓
//! BackendEvent::TextInput → egui::Event::Text
//! BackendEvent::Input    → egui::Event::Touch / PointerButton / etc.
//! BackendEvent::Lifecycle → Application::on_resume/on_pause/etc.
//!       ↓
//! egui::RawInput
//!       ↓
//! Application::frame()
//!       ↓
//! egui_glow рендеринг

#![cfg(target_os = "android")]

use std::sync::Arc;
use std::time::{Duration, Instant};

use android_activity::AndroidApp;

use crate::backend::{
    AndroidBackend, AndroidBackendKind, BackendEvent, InputEvent, LifecycleEvent,
};

use crate::input::InputState;

use egui_android_runtime::{
    ui_notifier::{AndroidWakeHandle, UiNotifier},
    Application,
};

use glow::HasContext;

/// Запустить egui-приложение на Android.
///
/// Использует `GlBackend` (основной).
pub fn run<A: Application>(app: AndroidApp) {
    run_with_backend::<A>(app, AndroidBackendKind::Gl);
}

/// Запустить egui-приложение с указанным backend'ом.
pub fn run_with_backend<A: Application>(app: AndroidApp, kind: AndroidBackendKind) {
    let mut app_instance = A::create();

    android_logger::init_once(
        android_logger::Config::default()
            .with_tag(app_instance.config().log_tag.as_str())
            .with_max_level(log::LevelFilter::Debug),
    );
    log::info!("run: запуск egui-android-framework, backend={:?}", kind);

    app_instance.on_create();
    app_instance.on_start();
    app_instance.on_resume();

    // --- Устанавливаем прозрачные системные бары (на Java main thread) ---
    // Важно: setStatusBarColor/setNavigationBarColor должны вызываться на UI thread.
    // Захватываем указатели до перемещения app в backend.
    let vm_ptr = app.vm_as_ptr() as usize;
    let activity_ptr = app.activity_as_ptr() as usize;
    app.run_on_java_main_thread(Box::new(move || {
        let vm = vm_ptr as *mut std::ffi::c_void;
        let activity = activity_ptr as *mut std::ffi::c_void;
        if !vm.is_null() && !activity.is_null() {
            crate::system_bars::set_transparent_system_bars(vm, activity);
        }
    }));

    // --- Создаём backend ---
    let mut backend: Box<dyn AndroidBackend> = match kind {
        AndroidBackendKind::Gl => {
            log::info!("run: создаём GlBackend");
            Box::new(crate::backend::GlBackend::new(app))
        }
        AndroidBackendKind::Native => {
            log::info!("run: создаём NativeBackend (fallback)");
            Box::new(crate::backend::NativeBackend::new(app))
        }
        AndroidBackendKind::Game => {
            log::warn!("GameBackend ещё не реализован, используем GlBackend");
            Box::new(crate::backend::GlBackend::new(app))
        }
    };

    // Backend будет инициализирован при первом InitWindow
    let egui_ctx = egui::Context::default();
    egui_ctx.set_pixels_per_point(backend.dpi());
    egui_ctx.set_fonts(egui::FontDefinitions::default());

    let mut egui_painter: Option<egui_glow::Painter> = None;
    let mut input_state = InputState::new();
    let mut last_frame = Instant::now();
    let target_dt = Duration::from_secs_f64(1.0 / app_instance.config().target_fps as f64);

    // Waker — создаём один раз из AndroidApp
    // После создания backend'а app больше не доступен напрямую.
    // Используем заглушку для waker'а (будет улучшено).
    let waker = AndroidWakeHandle::new(|| {});

    // UiNotifier
    let mut notifier: Option<UiNotifier> = None;
    let mut notifier_initialized = false;

    let mut destroy_requested = false;

    loop {
        // --- Получаем события от backend'а ---
        let backend_events = backend.poll_events();

        for event in backend_events {
            match event {
                BackendEvent::Lifecycle(ev) => match ev {
                    LifecycleEvent::InitWindow => {
                        log::info!("Lifecycle: InitWindow");

                        // Инициализируем backend (если ещё не инициализирован)
                        if let Err(e) = backend.init() {
                            log::error!("Ошибка инициализации backend'а: {}", e);
                        }

                        // Инициализируем глобальные указатели для JNI
                        let vm = backend.vm_ptr();
                        let activity = backend.activity_ptr();
                        crate::system_bars::init_global_pointers(vm, activity);
                        crate::backend::init_ime_globals(vm, activity);

                        // Системные бары уже были установлены через run_on_java_main_thread до создания backend'а

                        // Обновляем DPI
                        egui_ctx.set_pixels_per_point(backend.dpi());
                        // Painter будет создан ниже
                    }
                    LifecycleEvent::Resume => {
                        log::info!("Lifecycle: Resume");
                        app_instance.on_resume();
                    }
                    LifecycleEvent::Pause => {
                        log::info!("Lifecycle: Pause");
                        app_instance.on_pause();
                    }
                    LifecycleEvent::Stop => {
                        log::info!("Lifecycle: Stop");
                        app_instance.on_stop();
                    }
                    LifecycleEvent::Destroy => {
                        log::info!("Lifecycle: Destroy");
                        destroy_requested = true;
                    }
                },
                BackendEvent::Input(input_ev) => match input_ev {
                    InputEvent::Touch { phase, pos } => {
                        let egui_phase = match phase {
                            crate::backend::TouchPhase::Start => egui::TouchPhase::Start,
                            crate::backend::TouchPhase::Move => egui::TouchPhase::Move,
                            crate::backend::TouchPhase::End
                            | crate::backend::TouchPhase::Cancel => egui::TouchPhase::End,
                        };
                        input_state.events.push(egui::Event::Touch {
                            device_id: egui::TouchDeviceId(0),
                            id: egui::TouchId(0),
                            phase: egui_phase,
                            pos,
                            force: None,
                        });
                        match phase {
                            crate::backend::TouchPhase::Start => {
                                input_state.pointer_pos = Some(pos);
                            }
                            crate::backend::TouchPhase::End
                            | crate::backend::TouchPhase::Cancel => {
                                input_state.pointer_pos = None;
                            }
                            _ => {}
                        }
                    }
                    InputEvent::PointerButton { pos, pressed } => {
                        input_state.events.push(egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::default(),
                        });
                        if pressed {
                            input_state.pointer_pos = Some(pos);
                        } else {
                            input_state.pointer_pos = None;
                        }
                    }
                    InputEvent::Key { key_code, action } => {
                        if key_code == 4 // AKEYCODE_BACK = 4
                            && matches!(action, crate::backend::KeyAction::Down)
                        {
                            input_state.back_pressed = true;
                        }
                    }
                },
                BackendEvent::TextInput(text) => {
                    log::info!("IME: текстовый ввод: '{}'", &text);
                    // TextInput добавляется в input_state.events как Event::Text
                    input_state.events.push(egui::Event::Text(text));
                }
                BackendEvent::InsetsChanged(insets) => {
                    log::info!("InsetsChanged: {:?}", insets);
                    // Insets будут применены через screen_rect в следующем кадре
                }
                BackendEvent::DpiChanged(dpi) => {
                    log::info!("DpiChanged: {}", dpi);
                    egui_ctx.set_pixels_per_point(dpi);
                }
            }
        }

        // --- Проверка завершения ---
        if destroy_requested {
            app_instance.on_destroy();
            if let Some(ref mut painter) = egui_painter {
                painter.destroy();
            }
            if let Some(ref mut state) = backend.egl_state_mut() {
                state.destroy();
            }
            log::info!("Выход из главного цикла");
            break;
        }

        // --- Инициализация Painter (если ещё не создан) ---
        if egui_painter.is_none() {
            if let Some(ref _egl_state) = backend.egl_state() {
                let gl = unsafe {
                    egui_glow::painter::Context::from_loader_function(|name| {
                        let cname =
                            std::ffi::CStr::from_ptr(name.as_ptr() as *const std::os::raw::c_char);
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
                        egui_painter = Some(painter);

                        if !notifier_initialized {
                            notifier_initialized = true;
                            log::info!("run: инициализируем UiNotifier");
                            notifier = app_instance.create_notifier(&egui_ctx, waker.clone());
                        }
                    }
                    Err(e) => {
                        log::error!("Painter::new не удался: {:?}", e);
                    }
                }
            } else {
                // Нет EGL — ждём InitWindow
                continue;
            }
        }

        // --- Back ---
        if input_state.back_pressed {
            log::info!("Back нажата — отправляем в Application");
            input_state.back_pressed = false;

            // Если IME открыта — закрываем, иначе — навигация
            if app_instance.is_keyboard_visible() {
                app_instance.hide_keyboard();
                backend.hide_keyboard();
            } else {
                app_instance.on_back_pressed();
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

            let (w, h) = backend.window_size();
            if w == 0 || h == 0 {
                continue;
            }

            let pp = egui_ctx.pixels_per_point();

            // Получаем insets для этого кадра
            let insets = get_current_insets(&backend, pp, w, h);

            let events_for_frame = std::mem::take(&mut input_state.events);

            let screen_rect = egui::Rect::from_min_size(
                egui::Pos2::new(insets.left, insets.top),
                egui::vec2(
                    (w as f32 / pp) - insets.left - insets.right,
                    (h as f32 / pp) - insets.top - insets.bottom,
                ),
            );

            let predicted_dt = target_dt.as_secs_f64() as f32;
            let time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64();

            let raw_input = egui::RawInput {
                screen_rect: Some(screen_rect),
                events: events_for_frame,
                predicted_dt,
                time: Some(time),
                ..egui::RawInput::default()
            };

            let full_output = app_instance.frame(&egui_ctx, raw_input);

            if app_instance.request_destroy() {
                destroy_requested = true;
                continue;
            }

            // Рендеринг через egui_glow
            if let Some(ref mut painter) = egui_painter {
                unsafe {
                    let gl = &*painter.gl();

                    gl.disable(glow::SCISSOR_TEST);
                    let (cr, cg, cb) = crate::theme::current_clear_color();
                    gl.clear_color(cr, cg, cb, 1.0);
                    gl.clear(glow::COLOR_BUFFER_BIT);

                    let scissor_x = (insets.left * pp) as i32;
                    let scissor_y = (insets.bottom * pp) as i32;
                    let scissor_w = (w as f32 - insets.left * pp - insets.right * pp) as i32;
                    let scissor_h = (h as f32 - insets.top * pp - insets.bottom * pp) as i32;
                    gl.enable(glow::SCISSOR_TEST);
                    gl.scissor(scissor_x, scissor_y, scissor_w.max(1), scissor_h.max(1));
                }

                let primitives =
                    egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);

                painter.paint_and_update_textures(
                    [w, h],
                    full_output.pixels_per_point,
                    &primitives,
                    &full_output.textures_delta,
                );
            }

            // Swap buffers
            if let Some(ref state) = backend.egl_state() {
                if let Err(e) = state.swap_buffers() {
                    log::warn!("swap_buffers: {}", e);
                    if let Some(ref mut p) = egui_painter {
                        p.destroy();
                    }
                    egui_painter = None;
                }
            }
        }
    }
}

/// Получить текущие insets для кадра.
fn get_current_insets(
    _backend: &Box<dyn AndroidBackend>,
    pp: f32,
    _w: u32,
    _h: u32,
) -> crate::backend::Insets {
    // Пробуем загрузить insets из JNI (глобальные переменные)
    let jni_insets = crate::insets::load_insets_px();
    if jni_insets.is_valid() {
        return crate::backend::Insets {
            left: jni_insets.left as f32 / pp,
            top: jni_insets.top as f32 / pp,
            right: jni_insets.right as f32 / pp,
            bottom: jni_insets.bottom as f32 / pp,
        };
    }

    // Возвращаем insets из backend (если они были установлены)
    crate::backend::Insets::default()
}

/// Получить адрес OpenGL функции через dlsym.
fn get_proc_address(name: &std::ffi::CStr) -> *const std::ffi::c_void {
    // На Android нужно использовать eglGetProcAddress для загрузки OpenGL ES функций.
    // dlsym(RTLD_DEFAULT) не находит GL функции, экспортируемые драйвером.
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
