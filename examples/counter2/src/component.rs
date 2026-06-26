//! Компонент счётчика.
//!
//! Получает состояние из `StateStore` реактивно.
//! Не хранит каналы — данные приходят через store.

use egui_android_framework::{Component, LifecycleObserver};

use crate::msg::Msg;
use crate::view::counter_view;

/// Компонент счётчика: хранит состояние.
pub struct CounterComponent {
    pub count: u32,
}

impl LifecycleObserver for CounterComponent {}

impl Component for CounterComponent {
    type State = u32;
    type Message = Msg;

    fn render(&self, ui: &mut egui::Ui) -> Vec<Self::Message> {
        counter_view(&self.count, ui)
    }

    fn handle(&mut self, msg: Self::Message) {
        match msg {
            Msg::Increment => {
                log::info!("Component: handle Increment — отправляем в data layer");
                // Данные обновятся реактивно через StateStore
            }
        }
    }

    fn state(&self) -> &Self::State {
        &self.count
    }
}
