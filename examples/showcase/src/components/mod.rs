//! ScreenComponent — enum-агрегатор всех экранов.
//!
//! Каждый экран — это структура, которая реализует простой render-метод,
//! получая `Dispatcher<RootMsg>` для навигации и управления.
//!
//! # Передача ComponentContext
//!
//! Экран, которому нужен доступ к BackDispatcher (например, NestedScreen),
//! получает `ComponentContext` при создании через `from_route_with_ctx()`.

use egui_android_framework::core::{Component, ComponentContext, LifecycleObserver, UiWrapper};
use egui_android_framework::runtime::Dispatcher;

use crate::app::AppState;
use crate::navigation::Route;
use crate::root_component::RootMsg;
use crate::screens::{
    animations::AnimationsScreen, containers::ContainersScreen, home::HomeScreen,
    modifier_value::ModifierValueScreen, modifiers::ModifiersScreen, nested::NestedScreen,
    state_screen::StateScreen, themes::ThemesScreen, widgets::WidgetsScreen,
};

/// Единый enum для всех экранов.
pub enum ScreenComponent {
    Home(HomeScreen),
    Widgets(WidgetsScreen),
    Modifiers(ModifiersScreen),
    Containers(ContainersScreen),
    Themes(ThemesScreen),
    StateRef(StateScreen),
    Animations(AnimationsScreen),
    ModifierValue(ModifierValueScreen),
    Nested(NestedScreen),
}

impl ScreenComponent {
    /// Создать домашний экран.
    pub fn home() -> Self {
        Self::Home(HomeScreen::new())
    }

    /// Создать экран по маршруту с контекстом.
    ///
    /// Передаёт `ComponentContext` экранам, которым нужен BackDispatcher
    /// (NestedScreen). Остальные экраны не используют контекст.
    pub fn from_route_with_ctx(
        route: &Route,
        ctx: &mut ComponentContext<RootMsg, (), AppState>,
    ) -> Self {
        match route {
            Route::Home => Self::home(),
            Route::Widgets => Self::Widgets(WidgetsScreen::new()),
            Route::Modifiers => Self::Modifiers(ModifiersScreen::new()),
            Route::Containers => Self::Containers(ContainersScreen::new()),
            Route::Themes => Self::Themes(ThemesScreen::new()),
            Route::State => Self::StateRef(StateScreen::new()),
            Route::Animations => Self::Animations(AnimationsScreen::new()),
            Route::ModifierValue => Self::ModifierValue(ModifierValueScreen::new()),
            Route::Nested => Self::Nested(NestedScreen::new(ctx)),
            // NestedA/B/C — нельзя создать без контекста NestedScreen;
            // создаются через push_sub внутри NestedScreen
            Route::NestedA | Route::NestedB | Route::NestedC => {
                panic!("NestedA/B/C создаются через NestedScreen::push_sub, а не напрямую")
            }
        }
    }

    /// Создать экран по маршруту (без контекста).
    ///
    /// Используется для экранов без вложенной навигации.
    /// Для Nested экранов используйте `from_route_with_ctx()`.
    pub fn from_route(route: &Route) -> Self {
        match route {
            Route::Home => Self::home(),
            Route::Widgets => Self::Widgets(WidgetsScreen::new()),
            Route::Modifiers => Self::Modifiers(ModifiersScreen::new()),
            Route::Containers => Self::Containers(ContainersScreen::new()),
            Route::Themes => Self::Themes(ThemesScreen::new()),
            Route::State => Self::StateRef(StateScreen::new()),
            Route::Animations => Self::Animations(AnimationsScreen::new()),
            Route::ModifierValue => Self::ModifierValue(ModifierValueScreen::new()),
            Route::Nested => {
                panic!("Nested требует ComponentContext — используйте from_route_with_ctx()")
            }
            Route::NestedA | Route::NestedB | Route::NestedC => {
                panic!("NestedA/B/C создаются через NestedScreen::push_sub")
            }
        }
    }

    /// Пробросить тёмную тему.
    pub fn set_dark_mode(&mut self, is_dark: bool) {
        if let Self::Themes(ref mut screen) = self {
            screen.is_dark_mode = is_dark;
        }
    }

    /// Выполнить рендер экрана.
    pub fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<RootMsg>) {
        match self {
            Self::Home(s) => s.render(ui, dispatch),
            Self::Widgets(s) => s.render(ui, dispatch),
            Self::Modifiers(s) => s.render(ui, dispatch),
            Self::Containers(s) => s.render(ui, dispatch),
            Self::Themes(s) => s.render(ui, dispatch),
            Self::StateRef(s) => s.render(ui, dispatch),
            Self::Animations(s) => s.render(ui, dispatch),
            Self::ModifierValue(s) => s.render(ui, dispatch),
            Self::Nested(s) => s.render(ui, dispatch),
        }
    }

    /// Получить мутабельную ссылку на NestedScreen, если это он.
    pub fn as_nested_mut(&mut self) -> Option<&mut NestedScreen> {
        match self {
            Self::Nested(s) => Some(s),
            _ => None,
        }
    }
}

impl LifecycleObserver for ScreenComponent {}

// ScreenComponent реализует Component для совместимости с ChildStack.
// render() и handle() — пустые заглушки, реальный render через собственный метод.
impl Component for ScreenComponent {
    type State = ();
    type Message = ();

    fn render(&self, _ui: &mut egui::Ui, _dispatch: &Dispatcher<Self::Message>) {}

    fn handle(&mut self, _msg: Self::Message) {}

    fn state(&self) -> &Self::State {
        &()
    }
}
