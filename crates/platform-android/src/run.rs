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

                        // Если EGL уже инициализирован — пересоздаём surface (Pause/Resume)
                        if backend.egl_state().is_some() {
                            if let Err(e) = backend.recreate_surface() {
                                log::error!("Ошибка пересоздания EGL surface: {}", e);
                                // Уничтожаем painter — будет создан заново
                                if let Some(ref mut p) = egui_painter {
                                    p.destroy();
                                }
                                egui_painter = None;
                                if let Some(ref mut s) = backend.egl_state_mut() {
                                    s.destroy();
                                }
                            } else {
                                // Повторно инициализируем painter с новым surface
                                if let Some(ref mut p) = egui_painter {
                                    unsafe {
                                        let gl = &*p.gl();
                                        gl.clear_color(0.0, 0.0, 0.0, 1.0);
                                        gl.clear(glow::COLOR_BUFFER_BIT);
                                    }
                                }
                            }
                        } else {
                            // Первый InitWindow — инициализируем backend
                            if let Err(e) = backend.init() {
                                log::error!("Ошибка инициализации backend'а: {}", e);
                            }
                        }

                        // Инициализируем глобальные указатели для JNI
                        let vm = backend.vm_ptr();
                        let activity = backend.activity_ptr();
                        crate::system_bars::init_global_pointers(vm, activity);
                        crate::backend::init_ime_globals(vm, activity);

                        // Получаем WindowInsets через JNI
                        if !vm.is_null() && !activity.is_null() {
                            if let Some(insets) = crate::insets::get_system_insets_jni(vm, activity)
                            {
                                crate::insets::store_insets(&insets);
                                log::info!(
                                    "JNI insets: left={}, top={}, right={}, bottom={}",
                                    insets.left,
                                    insets.top,
                                    insets.right,
                                    insets.bottom
                                );
                            }
                        }

                        // Обновляем DPI
                        egui_ctx.set_pixels_per_point(backend.dpi());
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
                        // Для End/Cancel используем последнюю известную позицию,
                        // так как backend шлёт Pos2::ZERO для этих фаз.
                        let actual_pos = match phase {
                            crate::backend::TouchPhase::End
                            | crate::backend::TouchPhase::Cancel => {
                                input_state.pointer_pos.unwrap_or(pos)
                            }
                            _ => pos,
                        };
                        let egui_touch_phase = match phase {
                            crate::backend::TouchPhase::Start => egui::TouchPhase::Start,
                            crate::backend::TouchPhase::Move => egui::TouchPhase::Move,
                            crate::backend::TouchPhase::End
                            | crate::backend::TouchPhase::Cancel => egui::TouchPhase::End,
                        };
                        input_state.events.push(egui::Event::Touch {
                            device_id: egui::TouchDeviceId(0),
                            id: egui::TouchId(0),
                            phase: egui_touch_phase,
                            pos: actual_pos,
                            force: None,
                        });
                        // Для Move дополнительно шлём PointerMoved — без него egui
                        // не отслеживает позицию указателя, и скролл не работает.
                        if matches!(phase, crate::backend::TouchPhase::Move) {
                            input_state
                                .events
                                .push(egui::Event::PointerMoved(actual_pos));
                        }

                        match phase {
                            crate::backend::TouchPhase::Start
                            | crate::backend::TouchPhase::Move => {
                                input_state.pointer_pos = Some(pos);
                            }
                            crate::backend::TouchPhase::End
                            | crate::backend::TouchPhase::Cancel => {
                                // Не сбрасываем pointer_pos — он нужен для PointerButton UP ниже
                            }
                        }
                    }
                    InputEvent::PointerButton { pos, pressed } => {
                        // Для UP используем последнюю известную позицию
                        let actual_pos = if pressed {
                            pos
                        } else {
                            input_state.pointer_pos.unwrap_or(pos)
                        };
                        input_state.events.push(egui::Event::PointerButton {
                            pos: actual_pos,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::default(),
                        });
                        if pressed {
                            input_state.pointer_pos = Some(pos);
                        } else {
                            // Сбрасываем pointer_pos только после UP
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
///
/// Приоритет:
/// 1. JNI (WindowInsets.Type.systemBars) — глобальные переменные из insets.rs
/// 2. Fallback: content_rect из backend с clamp
fn get_current_insets(
    backend: &Box<dyn AndroidBackend>,
    pp: f32,
    w: u32,
    h: u32,
) -> crate::backend::Insets {
    // 1. Пробуем JNI insets
    let jni_insets = crate::insets::load_insets_px();
    if jni_insets.is_valid() {
        return crate::backend::Insets {
            left: jni_insets.left as f32 / pp,
            top: jni_insets.top as f32 / pp,
            right: jni_insets.right as f32 / pp,
            bottom: jni_insets.bottom as f32 / pp,
        };
    }

    // 2. Fallback: content_rect из backend
    let (left_px, top_px, right_px, bottom_px) = backend.content_rect();
    let top_pt = (top_px as f32 / pp).clamp(0.0, 32.0);
    let bottom_pt = ((h as i32 - bottom_px) as f32 / pp).clamp(0.0, 48.0);
    let left_pt = (left_px as f32 / pp).clamp(0.0, 32.0);
    let right_pt = ((w as i32 - right_px) as f32 / pp).clamp(0.0, 32.0);

    log::info!(
        "Insets fallback: content_rect=({},{},{},{}) px -> left={:.1} top={:.1} right={:.1} bottom={:.1} pt (pp={:.2})",
        left_px, top_px, right_px, bottom_px,
        left_pt, top_pt, bottom_pt, right_pt, pp
    );

    crate::backend::Insets {
        left: left_pt,
        top: top_pt,
        right: right_pt,
        bottom: bottom_pt,
    }
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
