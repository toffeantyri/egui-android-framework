//! Компонент счётчика.

use egui_android_framework::{Component, LifecycleObserver};

use crate::msg::{Evt, Msg};
use crate::view::counter_view;

/// Компонент счётчика: хранит состояние, не знает о каналах.
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
            }
        }
    }

    fn state(&self) -> &Self::State {
        &self.count
    }
}

/// Собственные методы CounterComponent (не из трейта Component).
impl CounterComponent {
    /// Обновить состояние из события data layer.
    pub fn apply_event(&mut self, evt: Evt) {
        match evt {
            Evt::CountUpdated(n) => {
                log::info!("Component: получено CountUpdated({n})");
                self.count = n;
            }
        }
    }
}
