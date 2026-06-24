//! Обработка Android-ввода: трансляция событий из `android-activity`
//! в события `egui` (touch, клавиатура, кнопки).
//!
//! Основная функция — [`process_input_events()`], которую вызывает
//! главный цикл `run()` один раз за кадр.

#![cfg(target_os = "android")]

use android_activity::{
    input::{InputEvent, MotionAction},
    InputStatus,
};
use ndk::native_window::NativeWindow;

/// Результат обработки ввода за кадр.
pub(crate) struct InputState {
    /// Накопленные события egui (pointer, keyboard и т.д.)
    pub(crate) events: Vec<egui::Event>,
    /// Текущая позиция указателя (если есть касание)
    pub(crate) pointer_pos: Option<egui::Pos2>,
}

impl InputState {
    pub(crate) fn new() -> Self {
        Self {
            events: Vec::new(),
            pointer_pos: None,
        }
    }

    pub(crate) fn clear(&mut self) {
        self.events.clear();
        // pointer_pos не сбрасываем — нужен для генерации PointerUp
    }
}

/// Обработать все накопившиеся события ввода из очереди Android.
///
/// Вызывается один раз за кадр. Заполняет `InputState` событиями egui.
///
/// `window` — опциональное native window (нужно для определения размеров
/// при трансляции координат, если потребуется).
#[allow(unused)]
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

/// Обработать одно событие ввода и вернуть `InputStatus`.
fn handle_input_event(event: InputEvent<'_>, pp: f32, state: &mut InputState) -> InputStatus {
    match event {
        InputEvent::MotionEvent(motion) => {
            let action = motion.action();
            let pointer = motion.pointers().next();
            match (action, pointer) {
                (MotionAction::Down, Some(p)) | (MotionAction::PointerDown, Some(p)) => {
                    let pos = egui::pos2(p.x() / pp, p.y() / pp);
                    state.pointer_pos = Some(pos);
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
                    if let Some(pos) = state.pointer_pos {
                        state.events.push(egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed: false,
                            modifiers: egui::Modifiers::default(),
                        });
                    }
                    state.pointer_pos = None;
                    InputStatus::Handled
                }
                (MotionAction::Move, Some(p)) => {
                    let pos = egui::pos2(p.x() / pp, p.y() / pp);
                    state.pointer_pos = Some(pos);
                    state.events.push(egui::Event::PointerMoved(pos));
                    InputStatus::Handled
                }
                _ => InputStatus::Unhandled,
            }
        }
        // TODO: клавиатура — InputEvent::KeyEvent
        // TODO: mouse / scroll — если Android-устройство с мышью
        _ => InputStatus::Unhandled,
    }
}
