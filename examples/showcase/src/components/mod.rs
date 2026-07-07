//! ScreenComponent — enum-агрегатор всех экранов.
//!
//! Каждый экран — это структура, которая реализует простой render-метод,
//! получая `Dispatcher<RootMsg>` для навигации и управления.

use egui_android_framework::core::{Component, LifecycleObserver, UiWrapper};
use egui_android_framework::runtime::Dispatcher;

use crate::navigation::Route;
use crate::root_component::RootMsg;
use crate::screens::{
    animations::AnimationsScreen, containers::ContainersScreen, home::HomeScreen,
    modifier_value::ModifierValueScreen, modifiers::ModifiersScreen, state_screen::StateScreen,
    themes::ThemesScreen, widgets::WidgetsScreen,
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
