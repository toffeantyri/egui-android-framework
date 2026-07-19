//! Обработка событий ввода: конвертация BackendEvent в egui::Event.
//!
//! Вынесена из `run.rs` для разделения ответственности.
//!
//! Содержит:
//! - [`process_backend_input`] — конвертация InputEvent/TextInput/InsetsChanged/DpiChanged
//! - [`process_back_pressed`] — обработка системной кнопки Back с учётом IME

#![cfg(target_os = "android")]

use crate::backend::AndroidBackend;
use crate::event::{BackendEvent, InputEvent, KeyAction, TouchPhase};
use crate::input::InputState;
use egui_android_runtime::Application;

/// Обработать событие от backend'а (кроме Lifecycle).
///
/// Поддерживаемые события:
/// - `InputEvent::Touch` — сенсорное событие → `egui::Event::Touch` + `PointerMoved`
/// - `InputEvent::PointerButton` — нажатие/отпускание → `egui::Event::PointerButton`
/// - `InputEvent::Key` — клавиша (только AKEYCODE_BACK) → `input_state.back_pressed`
/// - `TextInput` — IME текст → `egui::Event::Text`
/// - `InsetsChanged` — логирование (применяется в screen_rect на следующем кадре)
/// - `DpiChanged` — обновление pixels_per_point
///
/// Lifecycle-события обрабатываются в `crate::lifecycle::handle_lifecycle_event`.
pub fn process_backend_input(
    event: BackendEvent,
    input_state: &mut InputState,
    egui_ctx: &egui::Context,
) {
    match event {
        BackendEvent::Input(input_ev) => match input_ev {
            InputEvent::Touch { phase, pos } => {
                // Для End/Cancel используем последнюю известную позицию,
                // так как backend шлёт Pos2::ZERO для этих фаз.
                let actual_pos = match phase {
                    TouchPhase::End | TouchPhase::Cancel => input_state.pointer_pos.unwrap_or(pos),
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
            // Insets будут применены через screen_rect в следующем кадре
        }
        BackendEvent::DpiChanged(dpi) => {
            log::info!("DpiChanged: {}", dpi);
            egui_ctx.set_pixels_per_point(dpi);
        }
        // Lifecycle обрабатывается отдельно — сюда не должен попасть
        BackendEvent::Lifecycle(_) => {
            log::warn!("input_processing: получено Lifecycle-событие — игнорируем");
        }
    }
}

/// Обработать нажатие системной кнопки Back.
///
/// Если IME открыта — закрывает её.
/// Иначе — отправляет `on_back_pressed()` в Application.
pub fn process_back_pressed<A: Application>(
    app_instance: &mut A,
    backend: &mut dyn AndroidBackend,
    input_state: &mut InputState,
) {
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
