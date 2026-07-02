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

        Column::<RootMsg>::empty()
            .child(Spacer::new(16.0))
            .child(Text::new("Темы").padding(8.0))
            .child(Spacer::new(8.0))
            // Текущая тема
            .child(Text::new("Текущая тема:"))
            .child(
                Text::new(if self.is_dark_mode {
                    "Тёмная"
                } else {
                    "Светлая"
                })
                .padding(8.0)
                .background(theme.colors.primary),
            )
            .child(Spacer::new(8.0))
            // Палитра цветов
            .child(Text::new("Палитра цветов:"))
            .child(
                Text::new("Primary")
                    .padding(8.0)
                    .background(theme.colors.primary),
            )
            .child(
                Text::new("Secondary")
                    .padding(8.0)
                    .background(theme.colors.secondary),
            )
            .child(
                Text::new("Background")
                    .padding(8.0)
                    .background(theme.colors.background),
            )
            .child(
                Text::new("Surface")
                    .padding(8.0)
                    .background(theme.colors.surface),
            )
            .child(
                Text::new("Error")
                    .padding(8.0)
                    .background(theme.colors.error),
            )
            .child(Spacer::new(8.0))
            // Кнопка переключения темы
            .child(Text::new(
                "Переключение темы использует RootMsg::ToggleTheme:",
            ))
            .child(
                Button::new("Переключить тему")
                    .on_click(RootMsg::ToggleTheme)
                    .padding(8.0),
            )
            .child(Spacer::new(16.0))
            .child(Button::new("← Назад").on_click(RootMsg::Back).padding(8.0))
            .render(ui, dispatch);
    }
}
