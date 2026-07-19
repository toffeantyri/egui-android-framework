//! Главный цикл для Decompose-style архитектуры.
//!
//! Событийный цикл — без polling, без фоновых потоков для UI.
//! `RuntimeContext` проверяется синхронно в главном потоке.
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
//! GraphicsPipeline::render_frame()

#![cfg(target_os = "android")]

use std::time::{Duration, Instant};

use android_activity::AndroidApp;

use crate::backend::{AndroidBackend, AndroidBackendKind};
use crate::event::{BackendEvent, InputEvent, KeyAction, TouchPhase};
use crate::graphics::GraphicsPipeline;
use crate::input::InputState;

use egui_android_runtime::{Application, RuntimeContext};

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

    // --- Создаём backend и настраиваем системные бары ---
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

    let mut graphics: Option<GraphicsPipeline> = None;
    let mut input_state = InputState::new();
    let mut last_frame = Instant::now();
    let target_dt = Duration::from_secs_f64(1.0 / app_instance.config().target_fps as f64);

    // Waker — получаем из backend (вызывает app.wake() для реактивности)
    let waker = backend.create_waker();

    // RuntimeContext — владеет UiNotifier, инкапсулирует Waker
    let mut rt_ctx: Option<RuntimeContext> = None;
    let mut rt_ctx_initialized = false;

    let mut destroy_requested = false;

    loop {
        // --- Получаем события от backend'а ---
        let backend_events = backend.poll_events();

        for event in backend_events {
            match event {
                BackendEvent::Lifecycle(ev) => {
                    crate::lifecycle::handle_lifecycle_event(
                        ev,
                        &mut *backend,
                        &mut app_instance,
                        &egui_ctx,
                        &mut graphics,
                        &mut destroy_requested,
                    );
                }
                BackendEvent::Input(input_ev) => match input_ev {
                    InputEvent::Touch { phase, pos } => {
                        // Для End/Cancel используем последнюю известную позицию,
                        // так как backend шлёт Pos2::ZERO для этих фаз.
                        let actual_pos = match phase {
                            TouchPhase::End | TouchPhase::Cancel => {
                                input_state.pointer_pos.unwrap_or(pos)
                            }
                            _ => pos,
                        };
                        let egui_phase = match phase {
                            TouchPhase::Start => egui::TouchPhase::Start,
                            TouchPhase::Move => egui::TouchPhase::Move,
                            TouchPhase::End | TouchPhase::Cancel => egui::TouchPhase::End,
                        };
                        input_state.events.push(egui::Event::Touch {
                            device_id: egui::TouchDeviceId(0),
                            id: egui::TouchId(0),
                            phase: egui_phase,
                            pos: actual_pos,
                            force: None,
                        });
                        // Для Move дополнительно шлём PointerMoved — без него egui
                        // не отслеживает позицию указателя, и скролл не работает.
                        if matches!(phase, TouchPhase::Move) {
                            input_state
                                .events
                                .push(egui::Event::PointerMoved(actual_pos));
                        }

                        match phase {
                            TouchPhase::Start | TouchPhase::Move => {
                                input_state.pointer_pos = Some(pos);
                            }
                            TouchPhase::End | TouchPhase::Cancel => {
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
                            && matches!(action, KeyAction::Down)
                        {
                            input_state.back_pressed = true;
                        }
                    }
                },
                BackendEvent::TextInput(text) => {
                    log::info!("IME: текстовый ввод: '{}'", &text);
                    input_state.events.push(egui::Event::Text(text));
                }
                BackendEvent::InsetsChanged(insets) => {
                    log::info!("InsetsChanged: {:?}", insets);
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
            if let Some(ref mut g) = graphics {
                g.destroy();
            }
            backend.destroy_graphics();
            log::info!("Выход из главного цикла");
            break;
        }

        // --- Инициализация GraphicsPipeline (если ещё не создан) ---
        if graphics.is_none() {
            graphics = GraphicsPipeline::try_new(
                &mut *backend,
                &mut app_instance,
                &egui_ctx,
                &waker,
                &mut rt_ctx,
                &mut rt_ctx_initialized,
            );

            if graphics.is_none() {
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

        // --- Проверка уведомлений от data layer (через RuntimeContext) ---
        if let Some(ref mut ctx) = rt_ctx {
            ctx.check();
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

            // Рендеринг через GraphicsPipeline
            if let Some(ref mut g) = graphics {
                let clear_color = backend.platform_state().current_clear_color();
                let success = g.render_frame(
                    &egui_ctx,
                    &full_output,
                    (w, h),
                    clear_color,
                    (insets.left, insets.top, insets.right, insets.bottom),
                    pp,
                    &mut *backend,
                );
                if !success {
                    // swap_buffers не удался — пересоздадим painter
                    let mut p = None;
                    std::mem::swap(&mut p, &mut graphics);
                    if let Some(mut old) = p {
                        old.destroy();
                    }
                }
            }
        }
    }
}

/// Получить текущие insets для кадра.
///
/// Приоритет:
/// 1. JNI (WindowInsets.Type.systemBars) — через PlatformState в backend
/// 2. Fallback: content_rect из backend с clamp
fn get_current_insets(
    backend: &Box<dyn AndroidBackend>,
    pp: f32,
    w: u32,
    h: u32,
) -> crate::event::Insets {
    // 1. Пробуем JNI insets через PlatformState
    let state = backend.platform_state();
    if state.insets_are_valid() {
        let px = state.insets_px();
        return crate::event::Insets {
            left: px.left as f32 / pp,
            top: px.top as f32 / pp,
            right: px.right as f32 / pp,
            bottom: px.bottom as f32 / pp,
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

    crate::event::Insets {
        left: left_pt,
        top: top_pt,
        right: right_pt,
        bottom: bottom_pt,
    }
}
