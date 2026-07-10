//! RootComponent — корневой компонент с ChildStack и навигацией.
//!
//! Обработка Back:
//! - `ComponentContext::on_back()` — единая точка входа.
//! - Сначала `BackDispatcher` (NestedScreen может перехватить).
//! - Затем `back_fallback` (pop из ChildStack).
//! - После on_back() проверяем стек — если пуст, завершаем приложение.
//!
//! Системный Back (platform-android) и UI Back (кнопка "← Назад")
//! идут через один путь.

use egui_android_framework::{
    core::{Component, ComponentContext, LifecycleObserver, UiWrapper},
    navigation::ChildStack,
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
    /// Контекст компонента — центр навигации и обработки Back.
    pub context: ComponentContext<RootMsg, (), AppState>,
}

impl RootComponent {
    pub fn new(store: StateStore<AppState>) -> Self {
        let mut stack = ChildStack::new();
        let home = ScreenComponent::home();
        stack.push(Route::Home, home);

        // Создаём контекст без навигационного канала (root)
        // и без data_cmd (пока не используется)
        let (nav_tx, _nav_rx) = std::sync::mpsc::channel();
        let ctx = ComponentContext::new(None, nav_tx, store.clone_state());

        Self {
            stack,
            store,
            context: ctx,
        }
    }

    /// Настроить контекст: установить back_fallback.
    /// Вызывается после создания, т.к. back_fallback замыкается на self.
    pub fn setup_context(&mut self) {
        // Используем raw pointer для stack.
        // finish_requested не трогаем из замыкания — проверяем стек после on_back().
        struct RawStack(*mut ChildStack<Route, ScreenComponent>);
        unsafe impl Send for RawStack {}

        let raw = Box::new(RawStack(&mut self.stack as *mut _));
        let mut raw = Some(raw);

        let callback: Box<dyn FnMut() -> bool + Send> = Box::new(move || {
            let raw = raw.as_mut().unwrap();
            let stack = unsafe { &mut *raw.0 };

            if stack.is_empty() {
                return false;
            }

            if stack.len() == 1 {
                // Не делаем pop — на Home Back означает завершение.
                // finish_requested установит RootComponent после on_back().
                return false;
            }

            stack.pop();
            true
        });

        self.context.back_fallback = Some(callback);
    }

    pub fn sync_from_store(&mut self) {
        let app_state = self.store.state();
        if let Some(active) = self.stack.active_mut() {
            active.set_dark_mode(app_state.is_dark_mode);
        }
    }

    /// Обработать Back через ComponentContext и проверить стек.
    pub fn on_back(&mut self) {
        self.context.on_back();

        // Если стек пуст или остался только Home — завершаем приложение
        if self.stack.is_empty() || self.stack.len() == 1 {
            log::info!("RootComponent: стек пуст или Home — завершение приложения");
            self.context.finish_requested = true;
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
                // Nested требует ComponentContext для регистрации BackCallback
                let component = if route == Route::Nested {
                    ScreenComponent::from_route_with_ctx(&route, &mut self.context)
                } else {
                    ScreenComponent::from_route(&route)
                };
                self.stack.push(route, component);
            }
            RootMsg::Back => {
                // UI-кнопка назад — через ComponentContext (единый путь)
                self.on_back();
            }
            RootMsg::ToggleTheme => {
                self.store.update(|s| s.is_dark_mode = !s.is_dark_mode);
            }
        }
    }

    fn state(&self) -> &Self::State {
        panic!("state() не поддерживается для RootComponent")
    }
}
