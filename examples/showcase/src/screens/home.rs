//! HomeScreen — главный экран со списком демо-экранов, построенный на Compose-like API.

use egui_android_framework::{
    runtime::Dispatcher,
    ui::{
        containers::Column,
        modifier::ModifierExt,
        widgets::{Button, Spacer, Text, Widget},
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

    pub fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<RootMsg>) {
        let routes = vec![
            Route::Widgets,
            Route::Modifiers,
            Route::Containers,
            Route::Themes,
            Route::State,
            Route::Animations,
        ];

        Column::<RootMsg>::empty()
            .child(Spacer::new(30.0))
            .child(Text::new("Showcase").padding(8.0))
            .child(Spacer::new(16.0))
            .child(Text::new("Выберите демо:"))
            .child(
                routes
                    .into_iter()
                    .fold(Column::<RootMsg>::empty(), |col, route| {
                        col.child(
                            Button::new(route.title())
                                .on_click(RootMsg::Navigate(route))
                                .padding(8.0),
                        )
                    }),
            )
            .render(ui, dispatch);
    }
}
