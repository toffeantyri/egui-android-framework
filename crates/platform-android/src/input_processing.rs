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

/// Управляющий запрос от IME, требующий действия главного цикла (вне egui-событий).
///
/// Возвращается из [`process_ime_cmd`] для `performEditorAction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImeOutcome {
    /// `IME_ACTION_NEXT` — пользователь хочет перейти к следующему TextEdit.
    Next,
    /// `IME_ACTION_DONE/SEARCH/GO` — завершить редактирование, скрыть клавиатуру.
    Done,
    /// Обычное текстовое событие — специального действия не требуется.
    None,
}

/// Преобразовать IME-команду (из Kotlin InputConnection) в egui-события.
///
/// Возвращает [`ImeOutcome`] для управляющих действий (`Next`/`Done`), которые
/// главный цикл (`loop.rs`) обрабатывает отдельно. Текстовые команды
/// (`Commit`, `Composing`, `DeleteSurrounding`) кладутся в
/// `input_state.ime_pending` и доставляются в egui на **следующем** кадре
/// (двухкадровая доставка — защита от реентерабельной вставки внутри кадра).
///
/// # Содержительное преобразование
///
/// - `ImeCmd::Commit(text)` → `egui::Event::Text(text)` — вставка в фокусный TextEdit
/// - `ImeCmd::Composing(text)` → `egui::Event::Ime(ImeEvent::Preedit)` — preedit
/// - `ImeCmd::DeleteSurrounding{before,after}` → пары `Event::Key` (Backspace / Delete)
///
/// Никакого `InputEvent::TextEvent` / `setTextInputState` — только InputConnection.

pub fn process_ime_cmd(input_state: &mut InputState, cmd: crate::event::ImeCmd) -> ImeOutcome {
    use egui::Key;

    // Текстовые события кладём в `ime_pending`, а не в `events`: они будут
    // доставлены в egui на СЛЕДУЮЩЕМ кадре (см. `InputState::ime_pending` и
    // шаг 8 рендеринга в `loop.rs`). Это двухкадровая доставка IME-текста,
    // защищающая от реентерабельных вызовов изнутри активного кадра.
    //
    // Если активен batch (batch_depth > 0), события буферизуются в
    // `ime_batch_buffer` и выгружаются только при `endBatchEdit`.
    let pending = if input_state.ime_batch_depth > 0 {
        &mut input_state.ime_batch_buffer
    } else {
        &mut input_state.ime_pending
    };

    match cmd {
        crate::event::ImeCmd::Commit(text) => {
            // Commit финализирует IME-композицию: очищает активный preedit и
            // вставляет текст в позицию курсора (как IME-финал). Использование
            // `ImeEvent::Commit` (а не `Event::Text`) корректно завершает preedit,
            // когда перед commitText пришли setComposingText-кадры.
            log::info!(
                "IME: commitText -> {:?} (ImeEvent::Commit, next frame)",
                text
            );
            pending.push(egui::Event::Ime(egui::ImeEvent::Commit(text)));
            ImeOutcome::None
        }
        crate::event::ImeCmd::Composing(text) => {
            // Предредактируемый текст composition. Если строка пустая — IME
            // завершил preedit (active_range = None).
            log::info!("IME: setComposingText -> {:?} (Preedit, next frame)", text);
            let active_range_chars = if text.is_empty() {
                None
            } else {
                Some(0..text.chars().count())
            };
            pending.push(egui::Event::Ime(egui::ImeEvent::Preedit {
                text,
                active_range_chars,
            }));
            ImeOutcome::None
        }
        crate::event::ImeCmd::ComposingRange { text, start, end } => {
            log::info!("IME: setComposingText -> {:?} (Preedit, next frame)", text);
            let active_range_chars = if text.is_empty() {
                None
            } else {
                let char_start = utf16_offset_to_char_index(&text, start as usize);
                let char_end = utf16_offset_to_char_index(&text, end as usize);
                Some(char_start..char_end)
            };
            pending.push(egui::Event::Ime(egui::ImeEvent::Preedit {
                text,
                active_range_chars,
            }));
            ImeOutcome::None
        }
        crate::event::ImeCmd::Next => {
            log::info!("IME: IME_ACTION_NEXT");
            ImeOutcome::Next
        }
        crate::event::ImeCmd::Done => {
            log::info!("IME: IME_ACTION_DONE");
            ImeOutcome::Done
        }
        crate::event::ImeCmd::DeleteSurrounding { before, after } => {
            log::info!(
                "IME: deleteSurroundingText before={} after={} (Backspace/Delete, next frame)",
                before,
                after
            );
            // before → Backspace, after → Delete. Ограничиваем разумным числом.
            let before = before.clamp(0, 4);
            let after = after.clamp(0, 4);
            for _ in 0..before {
                pending.push(egui::Event::Key {
                    key: Key::Backspace,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::default(),
                });
            }
            for _ in 0..after {
                pending.push(egui::Event::Key {
                    key: Key::Delete,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::default(),
                });
            }
            ImeOutcome::None
        }
        crate::event::ImeCmd::SetSelection { start, end } => {
            // Пользователь переставил курсор/выделение в IME (например, через
            // touch или движение стрелками внутри клавиатуры). Полноценное
            // применение к egui-курсору требует обратного маппинга UTF-16 ->
            // символьные индексы и программной установки cursor range — в
            // текущей версии управляемо источником (egui сам обрабатывает
            // тапы/стрелки). Логируем и ничего не меняем.
            log::info!("IME: setSelection {start}..{end} (no-op, курсором управляет egui)");
            ImeOutcome::None
        }
        crate::event::ImeCmd::BeginBatchEdit => {
            input_state.ime_batch_depth += 1;
            log::debug!("IME: beginBatchEdit depth={}", input_state.ime_batch_depth);
            ImeOutcome::None
        }
        crate::event::ImeCmd::EndBatchEdit => {
            if input_state.ime_batch_depth > 0 {
                input_state.ime_batch_depth -= 1;
            }
            log::debug!("IME: endBatchEdit depth={}", input_state.ime_batch_depth);
            if input_state.ime_batch_depth == 0 {
                let buffered: Vec<egui::Event> = std::mem::take(&mut input_state.ime_batch_buffer);
                if !buffered.is_empty() {
                    log::debug!("IME: endBatchEdit — выгружено {} событий", buffered.len());
                    // Переносим накопленное в ime_pending (двухкадровая доставка)
                    input_state.ime_pending.extend(buffered);
                }
            }
            ImeOutcome::None
        }
    }
}

/// Обработать событие от backend'а (кроме Lifecycle).
///
/// Поддерживаемые события:
/// - `InputEvent::Touch` — сенсорное событие → `egui::Event::Touch` + `PointerMoved`
/// - `InputEvent::PointerButton` — нажатие/отпускание → `egui::Event::PointerButton`
/// - `InputEvent::Key` — клавиша (только AKEYCODE_BACK) → `input_state.back_pressed`
/// - `InsetsChanged` — логирование (применяется в screen_rect на следующем кадре)
/// - `DpiChanged` — обновление pixels_per_point
///
/// IME-текст НЕ обрабатывается здесь: он приходит через [`process_ime_cmd`]
/// (InputConnection → JNI → `ImeCmd`), см. модуль `ime_jni`.
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

/// Пересчитать UTF-16 offset в char-индекс (Unicode scalar value index).
///
/// Android IME передаёт позиции в UTF-16 code units (16-битные слова).
/// egui `active_range_chars` ожидает char-индексы (Unicode scalar values).
///
/// Для BMP-символов (кириллица, латиница, цифры) они совпадают.
/// Для non-BMP (эмодзи, некоторые иероглифы) UTF-16 единица = 2 суррогата,
/// поэтому char-индекс будет меньше.
fn utf16_offset_to_char_index(text: &str, utf16_offset: usize) -> usize {
    text.chars()
        .scan(0usize, |utf16_pos, ch| {
            let unit_len = ch.len_utf16();
            let current = *utf16_pos;
            *utf16_pos += unit_len;
            if current >= utf16_offset {
                None
            } else {
                Some(())
            }
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::ImeCmd;

    #[test]
    fn utf16_offset_bmp_chars_match() {
        assert_eq!(utf16_offset_to_char_index("hello", 3), 3);
        assert_eq!(utf16_offset_to_char_index("привет", 4), 4);
    }

    #[test]
    fn utf16_offset_zero_always_zero() {
        assert_eq!(utf16_offset_to_char_index("🎉🎉", 0), 0);
        assert_eq!(utf16_offset_to_char_index("", 0), 0);
    }

    #[test]
    fn utf16_offset_beyond_text_maps_to_len() {
        assert_eq!(utf16_offset_to_char_index("hi", 100), 2);
        assert_eq!(utf16_offset_to_char_index("", 5), 0);
    }

    #[test]
    fn batch_edit_buffers_events() {
        let mut input = InputState::new();

        // begin batch
        let outcome = process_ime_cmd(&mut input, ImeCmd::BeginBatchEdit);
        assert_eq!(outcome, ImeOutcome::None);
        assert_eq!(input.ime_batch_depth, 1);

        // commit внутри batch — в буфер
        process_ime_cmd(&mut input, ImeCmd::Commit("hello".into()));
        assert_eq!(input.ime_batch_buffer.len(), 1);
        assert!(input.ime_pending.is_empty());

        // composing внутри batch — тоже в буфер
        process_ime_cmd(&mut input, ImeCmd::Composing("w".into()));
        assert_eq!(input.ime_batch_buffer.len(), 2);

        // end batch — выгружает в pending
        process_ime_cmd(&mut input, ImeCmd::EndBatchEdit);
        assert_eq!(input.ime_batch_depth, 0);
        assert!(input.ime_batch_buffer.is_empty());
        assert_eq!(input.ime_pending.len(), 2);
    }

    #[test]
    fn nested_batch_flushes_only_at_depth_zero() {
        let mut input = InputState::new();

        process_ime_cmd(&mut input, ImeCmd::BeginBatchEdit);
        process_ime_cmd(&mut input, ImeCmd::BeginBatchEdit);
        assert_eq!(input.ime_batch_depth, 2);

        process_ime_cmd(&mut input, ImeCmd::Commit("x".into()));
        assert_eq!(input.ime_batch_buffer.len(), 1);

        // end внутреннего batch — НЕ выгружает
        process_ime_cmd(&mut input, ImeCmd::EndBatchEdit);
        assert_eq!(input.ime_batch_depth, 1);
        assert!(!input.ime_batch_buffer.is_empty());
        assert!(input.ime_pending.is_empty());

        // end внешнего batch — выгружает
        process_ime_cmd(&mut input, ImeCmd::EndBatchEdit);
        assert_eq!(input.ime_batch_depth, 0);
        assert!(input.ime_batch_buffer.is_empty());
        assert_eq!(input.ime_pending.len(), 1);
    }

    #[test]
    fn no_batch_goes_directly_to_pending() {
        let mut input = InputState::new();

        process_ime_cmd(&mut input, ImeCmd::Commit("test".into()));
        assert_eq!(input.ime_pending.len(), 1);
        assert!(input.ime_batch_buffer.is_empty());
    }
}
