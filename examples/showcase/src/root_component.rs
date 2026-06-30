//! RootComponent — корневой компонент с ChildStack и навигацией.

use egui_android_framework::{
    store::StateStore, ChildStack, Component, Dispatcher, LifecycleObserver,
};

use crate::app::AppState;
use crate::components::ScreenComponent;
use crate::navigation::Route;

/// Сообщения корневого компонента.
#[derive(Clone, Debug)]
pub enum RootMsg {
    /// Перейти на указанный маршрут.
    Navigate(Route),
    /// Вернуться назад.
    Back,
    /// Переключить тему.
    ToggleTheme,
}

/// Корневой компонент с навигацией.
pub struct RootComponent {
    pub stack: ChildStack<Route, ScreenComponent>,
    store: StateStore<AppState>,
}

impl RootComponent {
    pub fn new(store: StateStore<AppState>) -> Self {
        let mut stack = ChildStack::new();
        let home = ScreenComponent::home();
        stack.push(Route::Home, home);

        Self { stack, store }
    }

    pub fn sync_from_store(&mut self) {
        let app_state = self.store.state();
        if let Some(active) = self.stack.active_mut() {
            active.set_dark_mode(app_state.is_dark_mode);
        }
    }
}

impl LifecycleObserver for RootComponent {}

impl Component for RootComponent {
    type State = AppState;
    type Message = RootMsg;

    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<Self::Message>) {
        if let Some(active) = self.stack.active() {
            active.render(ui, dispatch);
        }
    }

    fn handle(&mut self, msg: Self::Message) {
        match msg {
            RootMsg::Navigate(route) => {
                let component = ScreenComponent::from_route(&route);
                self.stack.push(route, component);
            }
            RootMsg::Back => {
                self.stack.pop();
            }
            RootMsg::ToggleTheme => {
                self.store.update(|s| s.is_dark_mode = !s.is_dark_mode);
            }
        }
    }

    fn state(&self) -> &Self::State {
        // state() не используется напрямую для RootComponent
        // is_dark_mode передаётся через sync_from_store
        panic!("state() не поддерживается для RootComponent")
    }
}
