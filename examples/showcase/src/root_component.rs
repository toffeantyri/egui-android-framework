//! RootComponent — корневой компонент с ChildStack и навигацией.
//!
//! Обработка Back:
//! - Если активный компонент — BackCustomScreen, сначала пробуем его
//!   `handle_back()` (кастомная логика — переключение цвета).
//! - Если активный компонент — NestedScreen, затем пробуем его
//!   `handle_back()` (pop из вложенного стека).
//! - Затем `ComponentContext::on_back()` → `back_fallback` (pop из корневого
//!   ChildStack или завершение).
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
///
/// Хранит ChildStack в `Box`, чтобы указатель оставался стабильным
/// при перемещении RootComponent (из `new()` в поле Application).
/// `back_fallback` держит raw pointer на этот Box.
pub struct RootComponent {
    /// Корневой стек навигации. Box для стабильности указателя в back_fallback.
    pub stack: Box<ChildStack<Route, ScreenComponent>>,
    store: StateStore<AppState>,
    /// Контекст компонента — центр навигации и обработки Back.
    pub context: ComponentContext<RootMsg, (), AppState>,
}

impl RootComponent {
    pub fn new(store: StateStore<AppState>) -> Self {
        let (nav_tx, _nav_rx) = std::sync::mpsc::channel();
        let ctx = ComponentContext::new(None, nav_tx, store.clone_state());

        let stack = Box::new(ChildStack::new());

        let mut instance = Self {
            stack,
            store,
            context: ctx,
        };

        // Добавляем Home в корневой стек
        let home = ScreenComponent::home();
        instance.stack.push(Route::Home, home);

        // Устанавливаем back_fallback.
        // stack — Box, указатель на данные в куче стабилен при любых перемещениях Self.
        // finish_requested не трогаем из замыкания — проверяем стек после on_back().
        struct RawStack(*mut ChildStack<Route, ScreenComponent>);
        unsafe impl Send for RawStack {}

        let ptr = &mut *instance.stack as *mut ChildStack<Route, ScreenComponent>;
        let raw = Box::new(RawStack(ptr));
        let mut raw = Some(raw);

        let callback: Box<dyn FnMut() -> bool + Send> = Box::new(move || {
            let raw = raw.as_mut().unwrap();
            let stack = unsafe { &mut *raw.0 };

            if stack.is_empty() {
                return false;
            }

            if stack.len() == 1 {
                return false;
            }

            stack.pop();
            true
        });

        instance.context.back_fallback = Some(callback);

        instance
    }

    pub fn sync_from_store(&mut self) {
        let app_state = self.store.state();
        if let Some(active) = self.stack.active_mut() {
            active.set_dark_mode(app_state.is_dark_mode);
        }
    }

    /// Обработать Back.
    ///
    /// 1. BackCustomScreen::handle_back() — кастомная логика (переключение цвета).
    /// 2. NestedScreen::handle_back() — pop из вложенного стека.
    /// 3. ComponentContext::on_back() → back_fallback:
    ///    - если стек > 1 → pop (возврат на предыдущий экран)
    ///    - если стек = 1 (Home) → false (завершаем приложение)
    ///    - если стек = 0 → false (не должно быть)
    /// 4. После on_back(): если стек пуст — завершаем приложение.
    pub fn on_back(&mut self) {
        // Шаг 1: BackCustomScreen — кастомная логика (переключение цвета)
        if let Some(custom) = self.stack.active_mut().and_then(|c| c.as_back_custom_mut()) {
            if custom.handle_back() {
                return;
            }
        }
        // Шаг 2: NestedScreen — pop из вложенного стека
        if let Some(nested) = self.stack.active_mut().and_then(|c| c.as_nested_mut()) {
            if nested.handle_back() {
                return;
            }
        }

        // Шаг 3: BackDispatcher + back_fallback (pop из корневого стека или завершение)
        let len_before = self.stack.len();
        let handled = self.context.on_back();

        // Шаг 4: если стек не изменился и Back не обработан — Home, завершаем.
        if self.stack.is_empty() || (!handled && self.stack.len() == len_before) {
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
                // Создаём компонент по маршруту.
                // NestedScreen больше не регистрирует callback в BackDispatcher —
                // обработка Back идёт через прямой вызов handle_back() из on_back().
                let component = ScreenComponent::from_route(&route);
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
