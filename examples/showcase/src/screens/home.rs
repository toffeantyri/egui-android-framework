//! HomeScreen — главный экран со списком демо-экранов, построенный на Compose-like API.

use egui_android_framework::{
    runtime::Dispatcher,
    ui::{
        containers::Column,
        modifier::{Modifier, ModifierDsl},
        widgets::{Button, Spacer, Text, Widget},
        UiWrapper,
    },
};

use crate::navigation::Route;
use crate::root_component::RootMsg;

/// Главный экран.
pub struct HomeScreen;

impl HomeScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<RootMsg>) {
        let routes = vec![
            Route::Widgets,
            Route::Modifiers,
            Route::Containers,
            Route::Themes,
            Route::State,
            Route::Animations,
            Route::ModifierValue,
        ];

        Column::new()
            .scrollable()
            .show(ui, dispatch, |ui, dispatch| {
                Text::new("Showcase")
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);
                Spacer::new(16.0).render(ui, dispatch);
                Text::new("Выберите демо:").render(ui, dispatch);
                for route in routes {
                    Button::new(route.title())
                        .on_click(RootMsg::Navigate(route))
                        .modifier(Modifier::new().fill_max_width().padding(8.0))
                        .render(ui, dispatch);
                }
            });
    }
}
