//! [`ShowcaseFactory`] — фабрика экранов для showcase-приложения.
//!
//! Реализует [`ComponentFactory<Route>`], создавая соответствующий
//! компонент для каждого маршрута. Ранее эта логика была в `NavigationHost::create_screen()`.
//!
//! Теперь `NavigationHost` не знает про конкретные экраны — он принимает
//! фабрику через `Box<dyn ComponentFactory<Route>>`.

use egui_android_framework::{
    core::{ComponentNode, PersistentComponent},
    navigation::ComponentFactory,
};

use crate::navigation::Route;
use crate::screens::{
    animations::AnimationsScreen, back_custom::BackCustomScreen, containers::ContainersScreen,
    home::HomeScreen, modifier_value::ModifierValueScreen, nested::NestedScreen,
    state_screen::StateScreen, themes::ThemesScreen, widgets::WidgetsScreen,
};

/// Фабрика экранов showcase-приложения.
pub struct ShowcaseFactory;

impl ComponentFactory<Route> for ShowcaseFactory {
    fn create(&self, config: Route) -> Box<dyn ComponentNode> {
        match config {
            Route::Home => Box::new(HomeScreen::new()),
            Route::Widgets => Box::new(WidgetsScreen::new()),
            Route::Containers => Box::new(ContainersScreen::new()),
            Route::Themes => Box::new(ThemesScreen::new()),
            Route::State => {
                // StateScreen использует PersistentComponent для save/restore
                Box::new(PersistentComponent::new(StateScreen::new()))
            }
            Route::Animations => Box::new(AnimationsScreen::new()),
            Route::ModifierValue => Box::new(ModifierValueScreen::new()),
            Route::Nested => Box::new(NestedScreen::new()),
            Route::BackCustom => Box::new(BackCustomScreen::new()),
        }
    }
}
