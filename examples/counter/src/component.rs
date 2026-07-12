//! Компонент счётчика.
//!
//! Читает состояние из `StateStore` реактивно.

use egui_android_framework::{
    core::{Component, LifecycleObserver, UiWrapper},
    runtime::{Dispatcher, StateStore},
};

use crate::msg::{CounterState, Msg};
use crate::view::counter_view;

/// Компонент счётчика.
pub struct CounterComponent {
    /// Snapshot текущего состояния.
    count: u32,
    /// Реактивное состояние — источник истины.
    store: StateStore<CounterState>,
}

impl CounterComponent {
    pub fn new(store: StateStore<CounterState>) -> Self {
        let count = store.state().count;
        Self { count, store }
    }

    /// Синхронизировать count из store.
    pub fn sync_from_store(&mut self) {
        self.count = self.store.state().count;
    }

    pub fn get_count(&self) -> u32 {
        self.count
    }
}

impl LifecycleObserver for CounterComponent {}

impl Component for CounterComponent {
    type State = u32;
    type Message = Msg;

    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<Self::Message>) {
        let mut wrapper = UiWrapper::new_unconstrained(ui);
        counter_view(&self.count, &mut wrapper, dispatch)
    }

    fn handle(&mut self, msg: Self::Message) {
        match msg {
            Msg::Increment => {
                log::info!("Component: handle Increment — data layer обновит store");
            }
            Msg::ToggleTheme => {
                // ToggleTheme обрабатывается в Application::frame()
                log::info!("Component: ToggleTheme (обрабатывается на уровне Application)");
            }
        }
    }

    fn state(&self) -> &Self::State {
        &self.count
    }
}
