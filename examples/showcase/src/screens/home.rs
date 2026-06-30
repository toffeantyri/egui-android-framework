//! HomeScreen — главный экран со списком демо-экранов.

use egui_android_framework::{
    dispatcher::Dispatcher,
    widgets::{Button, Spacer, Text, Widget},
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
        let demos = [
            (Route::Widgets, "Виджеты", "Text, Button, Spacer, Icon"),
            (
                Route::Modifiers,
                "Модификаторы",
                "padding, size, background, align, clickable",
            ),
            (
                Route::Containers,
                "Контейнеры",
                "Column, Row, Stack, LazyColumn",
            ),
            (
                Route::Themes,
                "Темы",
                "MaterialTheme, ColorPalette, Typography",
            ),
            (
                Route::State,
                "Локальное состояние",
                "remember, RememberState",
            ),
            (
                Route::Animations,
                "Анимации",
                "AnimatedVisibility, Fade, Slide",
            ),
        ];

        Text::new("Showcase").render(ui, dispatch);
        Spacer::new(4.0).render(ui, dispatch);
        Text::new("Демонстрация возможностей фреймворка").render(ui, dispatch);
        Spacer::new(16.0).render(ui, dispatch);

        for (route, title, desc) in &demos {
            ui.separator();
            Text::new(*title).render(ui, dispatch);
            Text::new(*desc).render(ui, dispatch);
            Spacer::new(4.0).render(ui, dispatch);
            if ui.button("Открыть").clicked() {
                // dispatch через Dispatcher (RootMsg не Clone — используем кнопку)
            }
            // Через Button widget:
            Button::new("Открыть")
                .on_click(RootMsg::Navigate(route.clone()))
                .render(ui, dispatch);
            Spacer::new(8.0).render(ui, dispatch);
        }
    }
}
