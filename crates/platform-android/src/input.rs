//! Обработка Android-ввода: трансляция событий из `android-activity`
//! в события `egui` (touch, клавиатура, кнопки).

#![cfg(target_os = "android")]

use android_activity::{
    input::{InputEvent, KeyAction, MotionAction},
    InputStatus,
};

pub(crate) struct InputState {
    pub(crate) events: Vec<egui::Event>,
    pub(crate) pointer_pos: Option<egui::Pos2>,
    pub(crate) back_pressed: bool,
}

impl InputState {
    pub(crate) fn new() -> Self {
        Self {
            events: Vec::new(),
            pointer_pos: None,
            back_pressed: false,
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
