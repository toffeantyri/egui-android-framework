//! RootComponent — корневой компонент с ChildStack и навигацией.
//!
//! Обработка Back:
//! - Системный Back (платформенная кнопка) → `BackDispatcher.handle()`
//!   → зарегистрированные callback'и (NestedScreen, диалоги)
//!   → если никто не обработал → `pop()` из ChildStack
//! - UI Back (кнопка "← Назад") → `RootMsg::Back` → `handle(RootMsg::Back)`
//!   → проверяет BackDispatcher → если никто не обработал → `pop()`
//!
//! Единый путь: все Back сначала проходят через BackDispatcher.

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
    /// Флаг: запрошено завершение приложения (стек пуст).
    destroy_requested: bool,
}

impl RootComponent {
    pub fn new(store: StateStore<AppState>) -> Self {
        let mut stack = ChildStack::new();
        let home = ScreenComponent::home();
        stack.push(Route::Home, home);

        Self {
            stack,
            store,
            destroy_requested: false,
        }
    }

    pub fn sync_from_store(&mut self) {
        let app_state = self.store.state();
        if let Some(active) = self.stack.active_mut() {
            active.set_dark_mode(app_state.is_dark_mode);
        }
    }

    /// Получить флаг завершения.
    pub fn is_destroy_requested(&self) -> bool {
        self.destroy_requested
    }

    /// Обработать системную кнопку Back.
    ///
    /// Сначала проверяет активный компонент (NestedScreen, диалоги).
    /// Если компонент не перехватил Back:
    /// - Если стек пуст → ignore (не должно происходить)
    /// - Если в стеке 1 элемент (Home) → завершение приложения
    /// - Если в стеке > 1 элементов → pop
    pub fn handle_back(&mut self) {
        if self.stack.is_empty() {
            return;
        }

        // Проверяем активный компонент (NestedScreen может перехватить)
        let handled = self
            .stack
            .active_mut()
            .map(|c| c.handle_back())
            .unwrap_or(BackHandling::NotHandled);

        if handled == BackHandling::NotHandled {
            if self.stack.len() == 1 {
                // Home — последний экран, завершаем приложение
                log::info!("RootComponent: Back на Home — завершение приложения");
                self.destroy_requested = true;
            } else {
                self.stack.pop();
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
                // UI-кнопка назад: сначала проверяем активный компонент
                // Для единообразия используем ту же логику, что и системный Back
                self.handle_back();
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
