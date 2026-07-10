//! ScreenComponent — enum-агрегатор всех экранов.
//!
//! Каждый экран — это структура, которая реализует простой render-метод,
//! получая `Dispatcher<RootMsg>` для навигации и управления.
//!
//! Создание экранов — через `from_route()`.
//! NestedScreen не требует BackDispatcher — обработка Back идёт через
//! прямой вызов `handle_back()` из RootComponent::on_back().

use egui_android_framework::core::{Component, LifecycleObserver, UiWrapper};
use egui_android_framework::runtime::Dispatcher;

use crate::navigation::Route;
use crate::root_component::RootMsg;
use crate::screens::{
    animations::AnimationsScreen, back_custom::BackCustomScreen, containers::ContainersScreen,
    home::HomeScreen, modifier_value::ModifierValueScreen, modifiers::ModifiersScreen,
    nested::NestedScreen, state_screen::StateScreen, themes::ThemesScreen, widgets::WidgetsScreen,
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
    BackCustom(BackCustomScreen),
}

impl ScreenComponent {
    /// Создать домашний экран.
    pub fn home() -> Self {
        Self::Home(HomeScreen::new())
    }

    /// Создать экран по маршруту.
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
            Route::Nested => Self::Nested(NestedScreen::new()),
            // NestedA/B/C — нельзя создать без контекста NestedScreen;
            // создаются через push_sub внутри NestedScreen
            Route::NestedA | Route::NestedB | Route::NestedC => {
                panic!("NestedA/B/C создаются через NestedScreen::push_sub, а не напрямую")
            }
            Route::BackCustom => Self::BackCustom(BackCustomScreen::new()),
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
            Self::BackCustom(s) => s.render(ui, dispatch),
        }
    }

    /// Получить мутабельную ссылку на NestedScreen, если это он.
    pub fn as_nested_mut(&mut self) -> Option<&mut NestedScreen> {
        match self {
            Self::Nested(s) => Some(s),
            _ => None,
        }
    }

    /// Получить мутабельную ссылку на BackCustomScreen, если это он.
    pub fn as_back_custom_mut(&mut self) -> Option<&mut BackCustomScreen> {
        match self {
            Self::BackCustom(s) => Some(s),
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
