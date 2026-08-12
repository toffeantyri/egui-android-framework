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
use egui_android_runtime::{keyboard_controller_id, Application, KeyboardController};

// Входной тип команд IME и утилита перевода UTF-16 в char-индексы живут в
// `ime_logic` (без android-зависимостей). Сам редьюсер команд → egui-события
// перенесён в `ime_service`; здесь только re-export для `ime_jni`.
#[doc(inline)]
pub use crate::ime_logic::{utf16_offset_to_char_index, ImeCmd};

/// Обработать событие от backend'а (кроме Lifecycle).
///
/// Поддерживаемые события:
/// - `InputEvent::Touch` — сенсорное событие → `egui::Event::Touch` + `PointerMoved`
/// - `InputEvent::PointerButton` — нажатие/отпускание → `egui::Event::PointerButton`
/// - `InputEvent::Key` — клавиша (только AKEYCODE_BACK) → `input_state.back_pressed`
/// - `InsetsChanged` — логирование (применяется в screen_rect на следующем кадре)
/// - `DpiChanged` — обновление pixels_per_point
///
/// IME-текст НЕ обрабатывается здесь: он приходит из InputConnection → JNI →
/// `ImeCmd`, редьюсер — в `crate::ime_service` (см. модули `ime_jni`/`ime_service`).
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
    egui_ctx: &egui::Context,
) {
    log::info!("Back нажата — отправляем в Application");
    input_state.back_pressed = false;

    // Системный Back на Android скрывает клавиатуру на уровне системы (если IME
    // открыта), но не сообщает нам. Сбрасываем владельца клавиатуры БЕЗУСЛОВНО:
    // чтобы повторный тап на то же поле снова открыл клавиатуру. Иначе
    // `keyboard_is_owner` остаётся true, и show не перезапускается.
    egui_ctx.data(|d| {
        if let Some(kb) = d.get_temp::<KeyboardController>(keyboard_controller_id()) {
            *kb.owner_slot().write().unwrap() = None;
        }
    });

    // Снимаем egui-фокус с активного поля. Иначе поле остаётся фокусным, и в
    // следующем кадре `TextEdit::render` снова ставит себя владельцем и зовёт
    // `keyboard_show`, но Android не открывает клавиатуру (система только что её
    // закрыла Back). Снятие фокуса приводит к `lost_focus` на следующем кадре:
    // `TextEdit` убирает owner + publish_blur, и следующий тап полноценно
    // пере-показывает клавиатуру через `gained_focus`.
    egui_ctx.memory_mut(|m: &mut egui::Memory| {
        if let Some(focused_id) = m.focused() {
            m.surrender_focus(focused_id);
        }
    });

    // Если IME открыта — закрываем, иначе — навигация
    if app_instance.is_keyboard_visible() {
        app_instance.hide_keyboard();
        backend.hide_keyboard();
    } else {
        app_instance.on_back_pressed();
    }
}
