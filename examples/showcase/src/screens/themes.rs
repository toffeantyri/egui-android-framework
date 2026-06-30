//! ThemesScreen — демонстрация тем (light/dark).

use egui_android_framework::{
    dispatcher::Dispatcher,
    modifiers::ModifierExt,
    widgets::{Button, Spacer, Text, Widget},
    Theme,
};

use crate::root_component::RootMsg;

/// Экран демонстрации тем.
pub struct ThemesScreen {
    pub is_dark_mode: bool,
}

impl ThemesScreen {
    pub fn new() -> Self {
        Self {
            is_dark_mode: false,
        }
    }

    pub fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<RootMsg>) {
        let theme = Theme::current_from_ui(ui);

        Text::new("Темы").render(ui, dispatch);
        Spacer::new(8.0).render(ui, dispatch);

        Text::new("Текущая тема:").render(ui, dispatch);
        Text::new(if self.is_dark_mode {
            "Тёмная"
        } else {
            "Светлая"
        })
        .padding(8.0)
        .background(theme.colors.primary)
        .render(ui, dispatch);

        Spacer::new(8.0).render(ui, dispatch);
        Text::new("Палитра цветов:").render(ui, dispatch);

        let colors = [
            ("Primary", theme.colors.primary, theme.colors.on_primary),
            (
                "Secondary",
                theme.colors.secondary,
                theme.colors.on_secondary,
            ),
            (
                "Background",
                theme.colors.background,
                theme.colors.on_background,
            ),
            ("Surface", theme.colors.surface, theme.colors.on_surface),
            ("Error", theme.colors.error, theme.colors.on_error),
        ];

        for (name, bg, _fg) in &colors {
            Text::new(*name)
                .background(*bg)
                .padding(8.0)
                .render(ui, dispatch);
        }

        Spacer::new(8.0).render(ui, dispatch);
        Button::new("Переключить тему")
            .on_click(RootMsg::ToggleTheme)
            .render(ui, dispatch);

        Spacer::new(16.0).render(ui, dispatch);
        Button::new("Назад")
            .on_click(RootMsg::Back)
            .render(ui, dispatch);
    }
}
