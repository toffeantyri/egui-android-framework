//! NavigationHost — хост навигации с ChildStack.
//!
//! Координатор между Application и экранами.
//! Владеет корневым ChildStack, создаёт экраны по маршруту,
//! маршрутизирует сообщения, обрабатывает BackPressed.
//!
//! Не реализует Component трейт — рендер через render_dyn.
//!
//! Обработка Back:
//! 1. BackCustomScreen::handle_back() — кастомная логика (переключение цвета).
//! 2. NestedScreen::handle_back() — pop из вложенного стека.
//! 3. ComponentContext::on_back() → back_fallback:
//!    - стек > 1 → pop из корневого ChildStack
//!    - стек = 1 (Home) → false (завершение приложения)
//! 4. Если стек пуст или не изменился — завершение.
//!
//! Системный Back (platform-android) и UI Back (кнопка "← Назад")
//! идут через один путь.

use egui_android_framework::{
    core::{ComponentContext, ComponentNode, LifecycleObserver, UiWrapper},
    navigation::ChildStack,
    runtime::{DynDispatcher, StateStore},
};

use crate::app::AppState;
use crate::navigation::{NavigableRoute, Route};
use crate::screens::{
    animations::AnimationsScreen, back_custom::BackCustomScreen, containers::ContainersScreen,
    home::HomeScreen, modifier_value::ModifierValueScreen, nested::NestedScreen,
    state_screen::StateScreen, themes::ThemesScreen, widgets::WidgetsScreen,
};

/// Сообщения навигации.
#[derive(Clone, Debug)]
pub enum RootMsg {
    /// Перейти по маршруту (корневой стек или вложенный).
    Navigate(NavigableRoute),
    /// Вернуться назад (pop из стека).
    Back,
    /// Переключить тему.
    ToggleTheme,
}

/// Хост навигации — координатор между Application и экранами.
///
/// Владеет корневым ChildStack, ComponentContext, Store.
/// Не реализует Component — рендер через render_dyn().
pub struct NavigationHost {
    /// Корневой стек навигации. Box для стабильности указателя в back_fallback.
    pub stack: Box<ChildStack<Route>>,
    store: StateStore<AppState>,
    /// Контекст — центр навигации и обработки Back.
    pub context: ComponentContext<RootMsg, (), AppState>,
}

impl NavigationHost {
    pub fn new(store: StateStore<AppState>) -> Self {
        let (navevent_tx, _navevent_rx) = std::sync::mpsc::channel();
        let ctx = ComponentContext::new(None, navevent_tx, store.clone_state());

        let stack = Box::new(ChildStack::new());

        let mut instance = Self {
            stack,
            store,
            context: ctx,
        };

        // Добавляем Home в корневой стек
        instance
            .stack
            .push(Route::Home, Self::create_screen(&Route::Home));

        // Устанавливаем back_fallback.
        // stack — Box, указатель на данные в куче стабилен при любых перемещениях Self.
        // finish_requested не трогаем из замыкания — проверяем стек после on_back().
        struct RawStack(*mut ChildStack<Route>);
        unsafe impl Send for RawStack {}

        let ptr = &mut *instance.stack as *mut ChildStack<Route>;
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

    fn create_screen(route: &Route) -> Box<dyn ComponentNode> {
        match route {
            Route::Home => Box::new(HomeScreen::new()),
            Route::Widgets => Box::new(WidgetsScreen::new()),
            Route::Containers => Box::new(ContainersScreen::new()),
            Route::Themes => Box::new(ThemesScreen::new()),
            Route::State => Box::new(StateScreen::new()),
            Route::Animations => Box::new(AnimationsScreen::new()),
            Route::ModifierValue => Box::new(ModifierValueScreen::new()),
            Route::Nested => Box::new(NestedScreen::new()),
            Route::BackCustom => Box::new(BackCustomScreen::new()),
        }
    }

    pub fn sync_from_store(&mut self) {
        let app_state = self.store.state();
        // Пробрасываем is_dark_mode в ThemesScreen
        if let Some(active) = self.stack.active_mut() {
            if let Some(themes) = active.as_any_mut().downcast_mut::<ThemesScreen>() {
                themes.is_dark_mode = app_state.is_dark_mode;
            }
        }
    }

    /// Обработать Back.
    ///
    /// 1. BackCustomScreen::handle_back() — кастомная логика.
    /// 2. NestedScreen::handle_back() — pop из вложенного стека.
    /// 3. ComponentContext::on_back() → back_fallback.
    /// 4. Если стек не изменился — завершение.
    pub fn on_back(&mut self) {
        // Шаг 1: BackCustomScreen — кастомная логика (переключение цвета)
        if let Some(active) = self.stack.active_mut() {
            if let Some(custom) = active.as_any_mut().downcast_mut::<BackCustomScreen>() {
                if custom.handle_back() {
                    return;
                }
            }
        }
        // Шаг 2: NestedScreen — pop из вложенного стека
        if let Some(active) = self.stack.active_mut() {
            if let Some(nested) = active.as_any_mut().downcast_mut::<NestedScreen>() {
                if nested.handle_back() {
                    return;
                }
            }
        }

        // Шаг 3: BackDispatcher + back_fallback
        let len_before = self.stack.len();
        let handled = self.context.on_back();

        // Шаг 4: если стек не изменился и Back не обработан — Home, завершаем.
        if self.stack.is_empty() || (!handled && self.stack.len() == len_before) {
            self.context.finish_requested = true;
        }
    }

    pub fn handle_msg(&mut self, msg: RootMsg) {
        match msg {
            RootMsg::Navigate(nav) => match nav {
                NavigableRoute::Main(route) => {
                    self.stack.push(route.clone(), Self::create_screen(&route));
                }
                NavigableRoute::Nested(route) => {
                    if let Some(active) = self.stack.active_mut() {
                        if let Some(nested) = active.as_any_mut().downcast_mut::<NestedScreen>() {
                            nested.push_sub(route);
                        }
                    }
                }
            },
            RootMsg::Back => {
                self.on_back();
            }
            RootMsg::ToggleTheme => {
                self.store.update(|s| s.is_dark_mode = !s.is_dark_mode);
            }
        }
    }

    /// Рендеринг с DynDispatcher.
    pub fn render_dyn(&self, ui: &mut UiWrapper, uidynmsg_tx: &DynDispatcher) {
        if let Some(active) = self.stack.active() {
            active.render(ui, uidynmsg_tx);
        }
    }
}

impl LifecycleObserver for NavigationHost {}
