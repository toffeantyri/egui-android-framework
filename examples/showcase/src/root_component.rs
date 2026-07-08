//! RootComponent — корневой компонент с ChildStack и навигацией.

use std::sync::mpsc;

use egui_android_framework::{
    core::{Component, LifecycleObserver, UiWrapper},
    navigation::{BackHandling, ChildStack},
    runtime::Dispatcher,
    runtime::StateStore,
};

use crate::app::AppState;
use crate::components::ScreenComponent;
use crate::navigation::Route;

/// Сообщения корневого компонента.
#[derive(Clone, Debug)]
pub enum RootMsg {
    /// Перейти на указанный маршрут.
    Navigate(Route),
    /// Вернуться назад (pop из стека).
    Back,
    /// Переключить тему.
    ToggleTheme,
}

/// Корневой компонент с навигацией.
pub struct RootComponent {
    pub stack: ChildStack<Route, ScreenComponent>,
    store: StateStore<AppState>,
    /// Отправитель сигнала о завершении (когда стек пуст).
    finish_tx: Option<mpsc::Sender<()>>,
}

impl RootComponent {
    pub fn new(store: StateStore<AppState>) -> Self {
        let mut stack = ChildStack::new();
        let home = ScreenComponent::home();
        stack.push(Route::Home, home);

        Self {
            stack,
            store,
            finish_tx: None,
        }
    }

    /// Установить отправитель сигнала завершения.
    pub fn set_finish_tx(&mut self, tx: mpsc::Sender<()>) {
        self.finish_tx = Some(tx);
    }

    pub fn sync_from_store(&mut self) {
        let app_state = self.store.state();
        if let Some(active) = self.stack.active_mut() {
            active.set_dark_mode(app_state.is_dark_mode);
        }
    }

    /// Обработать системную кнопку Back (вызывается из Application).
    ///
    /// Если активный компонент не перехватил — делаем pop.
    /// Если стек стал пуст — сигнал завершения.
    pub fn handle_back(&mut self) {
        let handled = self
            .stack
            .active_mut()
            .map(|c| c.on_back())
            .unwrap_or(BackHandling::NotHandled);

        if handled == BackHandling::NotHandled {
            if self.stack.is_empty() {
                log::info!("RootComponent: стек пуст, завершение");
            } else {
                self.stack.pop();
                if self.stack.is_empty() {
                    log::info!("RootComponent: после pop стек пуст, завершение");
                    if let Some(ref finish_tx) = self.finish_tx {
                        let _ = finish_tx.send(());
                    }
                }
            }
        }
    }
}

impl LifecycleObserver for RootComponent {}

impl Component for RootComponent {
    type State = AppState;
    type Message = RootMsg;

    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<Self::Message>) {
        if let Some(active) = self.stack.active() {
            let mut wrapper = UiWrapper::new_unconstrained(ui);
            active.render(&mut wrapper, dispatch);
        }
    }

    fn handle(&mut self, msg: Self::Message) {
        match msg {
            RootMsg::Navigate(route) => {
                // Если это вложенный экран (NestedA/B/C) и активный компонент — NestedScreen,
                // то push во вложенный стек, а не в корневой.
                let is_nested_sub =
                    matches!(route, Route::NestedA | Route::NestedB | Route::NestedC);
                if is_nested_sub {
                    if let Some(nested) = self.stack.active_mut().and_then(|c| c.as_nested_mut()) {
                        nested.push_sub(route);
                        return;
                    }
                }
                let component = ScreenComponent::from_route(&route);
                self.stack.push(route, component);
            }
            RootMsg::Back => {
                // UI-кнопка назад: сначала спрашиваем активный компонент
                let handled = self
                    .stack
                    .active_mut()
                    .map(|c| c.on_back())
                    .unwrap_or(BackHandling::NotHandled);
                if handled == BackHandling::NotHandled {
                    self.stack.pop();
                    if self.stack.is_empty() {
                        log::info!("RootComponent: после pop стек пуст, завершение");
                        if let Some(ref finish_tx) = self.finish_tx {
                            let _ = finish_tx.send(());
                        }
                    }
                }
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
