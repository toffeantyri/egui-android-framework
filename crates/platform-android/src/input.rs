//! Обработка Android-ввода: трансляция событий из `android-activity`
//! в события `egui` (touch, клавиатура, кнопки).
//!
//! Поддерживает:
//! - Touch (MotionEvent)
//! - Back (KeyEvent)
//! - Текстовый ввод (через JNI, не через InputEvent::TextEvent)

#![cfg(target_os = "android")]

use android_activity::{
    input::{InputEvent, KeyAction, MotionAction},
    InputStatus,
};

pub struct InputState {
    pub events: Vec<egui::Event>,
    pub pointer_pos: Option<egui::Pos2>,
    pub back_pressed: bool,
    /// IME-события (`Event::Text`, preedit, Backspace/Delete), накопленные в
    /// текущем кадре через `process_ime_cmd`.
    ///
    /// # Двухкадровая доставка IME
    ///
    /// IME-текст НЕ доставляется в кадр, в котором он пришёл. Это защищает
    /// egui от повторной/многопроходной вставки текста и реентерабельных
    /// вызовов `remember().set()` изнутри активного прохода кадра (deferred-
    /// обработка, как в референсе egui-android). События из `ime_pending`
    /// попадают в `events` (и в `RawInput`) только на СЛЕДУЮЩЕМ кадре — см.
    /// `crate::loop::RunState::tick` (шаг 8 рендеринга).
    pub ime_pending: Vec<egui::Event>,
    /// Буфер доставки: содержит `ime_pending` из ПРОШЛОГО кадра, готовый к
    /// вставке в `RawInput` в текущем кадре.
    pub ime_deliver: Vec<egui::Event>,
    /// Глубина вложенности batch-операций IME (`beginBatchEdit`/`endBatchEdit`).
    /// Пока `batch_depth > 0`, текстовые события буферизуются в `ime_batch_buffer`.
    pub ime_batch_depth: u32,
    /// Буфер для накопления IME-событий внутри batch-операции. При `endBatchEdit`
    /// (когда `batch_depth` становится 0) содержимое переносится в `ime_pending`.
    pub ime_batch_buffer: Vec<egui::Event>,
}

impl InputState {
    pub(crate) fn new() -> Self {
        Self {
            events: Vec::new(),
            pointer_pos: None,
            back_pressed: false,
            ime_pending: Vec::new(),
            ime_deliver: Vec::new(),
            ime_batch_depth: 0,
            ime_batch_buffer: Vec::new(),
        }
    }
}

pub(crate) fn process_input_events(
    app: &android_activity::AndroidApp,
    pixels_per_point: f32,
    state: &mut InputState,
) {
    let Ok(mut iter) = app.input_events_iter() else {
        return;
    };

    loop {
        let has = iter.next(|event| handle_input_event(event, pixels_per_point, state));
        if !has {
            break;
        }
    }
}

fn handle_input_event(event: &InputEvent<'_>, pp: f32, state: &mut InputState) -> InputStatus {
    match event {
        InputEvent::MotionEvent(motion) => {
            let action = motion.action();
            let pointer = motion.pointers().next();
            match (action, pointer) {
                (MotionAction::Down, Some(p)) | (MotionAction::PointerDown, Some(p)) => {
                    let pos = egui::pos2(p.x() / pp, p.y() / pp);

                    state.pointer_pos = Some(pos);

                    // Для активации DragScroll::OnTouch в egui 0.35 нужно отправить
                    // хотя бы одно Event::Touch, чтобы has_touch_screen() вернула true.
                    state.events.push(egui::Event::Touch {
                        device_id: egui::TouchDeviceId(0),
                        id: egui::TouchId(0),
                        phase: egui::TouchPhase::Start,
                        pos,
                        force: None,
                    });

                    state.events.push(egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::default(),
                    });
                    InputStatus::Handled
                }
                (MotionAction::Up, _)
                | (MotionAction::PointerUp, _)
                | (MotionAction::Cancel, _) => {
                    if let Some(pos) = state.pointer_pos.take() {
                        state.events.push(egui::Event::Touch {
                            device_id: egui::TouchDeviceId(0),
                            id: egui::TouchId(0),
                            phase: egui::TouchPhase::End,
                            pos,
                            force: None,
                        });
                        state.events.push(egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed: false,
                            modifiers: egui::Modifiers::default(),
                        });
                    }
                    InputStatus::Handled
                }
                (MotionAction::Move, Some(p)) => {
                    let pos = egui::pos2(p.x() / pp, p.y() / pp);
                    state.pointer_pos = Some(pos);
                    state.events.push(egui::Event::Touch {
                        device_id: egui::TouchDeviceId(0),
                        id: egui::TouchId(0),
                        phase: egui::TouchPhase::Move,
                        pos,
                        force: None,
                    });
                    state.events.push(egui::Event::PointerMoved(pos));
                    InputStatus::Handled
                }
                _ => InputStatus::Unhandled,
            }
        }
        InputEvent::KeyEvent(key) => {
            let action = key.action();
            let code = key.key_code();
            if action == KeyAction::Down && code == android_activity::input::Keycode::Back {
                state.back_pressed = true;
                InputStatus::Handled
            } else {
                InputStatus::Unhandled
            }
        }
        _ => InputStatus::Unhandled,
    }
}
