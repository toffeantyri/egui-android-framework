//! HomeScreen — главный экран со списком демо-экранов.

use egui_android_framework::runtime::Dispatcher;

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

        ui.heading("Showcase");
        ui.add_space(4.0);
        ui.label("Демонстрация возможностей фреймворка");
        ui.add_space(16.0);

        for (route, title, desc) in &demos {
            ui.separator();
            ui.label(*title);
            ui.label(*desc);
            ui.add_space(4.0);
            ui.add_enabled_ui(false, |ui| if ui.button("Открыть").clicked() {});
            if ui.button("Открыть").clicked() {
                dispatch.dispatch(RootMsg::Navigate(route.clone()));
            }
            ui.add_space(8.0);
        }
    }
}
