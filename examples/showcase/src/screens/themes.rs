//! ThemesScreen — демонстрация тем (light/dark).

use egui_android_framework::{
    runtime::Dispatcher,
    ui::{
        containers::Column,
        modifier::ModifierExt,
        theme::{MaterialTheme, Theme},
        widgets::{Button, Spacer, Text, Widget},
    },
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
        // Применяем тему из фреймворка
        if self.is_dark_mode {
            MaterialTheme::dark().apply(ui.ctx());
        } else {
            MaterialTheme::light().apply(ui.ctx());
        }

        let theme = Theme::current(ui.ctx());

        Column::new(ui, dispatch, |ui, dispatch| {
            Text::new("Темы").padding(8.0).render(ui, dispatch);
            Spacer::new(8.0).render(ui, dispatch);

            // Текущая тема
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

            // Палитра цветов
            Text::new("Палитра цветов:").render(ui, dispatch);
            Text::new("Primary")
                .padding(8.0)
                .background(theme.colors.primary)
                .render(ui, dispatch);
            Text::new("Secondary")
                .padding(8.0)
                .background(theme.colors.secondary)
                .render(ui, dispatch);
            Text::new("Background")
                .padding(8.0)
                .background(theme.colors.background)
                .render(ui, dispatch);
            Text::new("Surface")
                .padding(8.0)
                .background(theme.colors.surface)
                .render(ui, dispatch);
            Text::new("Error")
                .padding(8.0)
                .background(theme.colors.error)
                .render(ui, dispatch);

            Spacer::new(8.0).render(ui, dispatch);

            // Кнопка переключения темы
            Text::new("Переключение темы использует RootMsg::ToggleTheme:").render(ui, dispatch);
            Button::new("Переключить тему")
                .on_click(RootMsg::ToggleTheme)
                .padding(8.0)
                .render(ui, dispatch);

            Spacer::new(16.0).render(ui, dispatch);
            Button::new("← Назад")
                .on_click(RootMsg::Back)
                .padding(8.0)
                .render(ui, dispatch);
        });
    }
}
