//! ThemesScreen — демонстрация тем (light/dark).

use egui_android_framework::{
    runtime::Dispatcher,
    ui::{
        containers::Column,
        modifier::{Modifier, ModifierDsl},
        theme::{MaterialTheme, Theme},
        widgets::{Button, Spacer, Text, Widget},
        UiWrapper,
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

    pub fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<RootMsg>) {
        // Применяем тему
        if self.is_dark_mode {
            MaterialTheme::dark().apply(ui.ctx());
        } else {
            MaterialTheme::light().apply(ui.ctx());
        }

        let theme = Theme::current(ui.ctx());
        let c = &theme.colors;

        Column::new()
            .scrollable()
            .show(ui, dispatch, |ui, dispatch| {
                Text::new("Темы")
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);
                Spacer::new(8.0).render(ui, dispatch);

                // Текущая тема
                Text::new("Текущая тема:").render(ui, dispatch);
                Text::new(if self.is_dark_mode {
                    "Тёмная"
                } else {
                    "Светлая"
                })
                .text_color(c.on_primary)
                .modifier(
                    Modifier::new()
                        .fill_max_width()
                        .padding(8.0)
                        .background(c.primary),
                )
                .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // Палитра цветов
                Text::new("Палитра цветов:").render(ui, dispatch);
                Text::new("Primary")
                    .text_color(c.on_primary)
                    .modifier(
                        Modifier::new()
                            .fill_max_width()
                            .padding(8.0)
                            .background(c.primary),
                    )
                    .render(ui, dispatch);
                Text::new("Secondary")
                    .text_color(c.on_secondary)
                    .modifier(
                        Modifier::new()
                            .fill_max_width()
                            .padding(8.0)
                            .background(c.secondary),
                    )
                    .render(ui, dispatch);
                Text::new("Background")
                    .text_color(c.on_background)
                    .modifier(
                        Modifier::new()
                            .fill_max_width()
                            .padding(8.0)
                            .background(c.background),
                    )
                    .render(ui, dispatch);
                Text::new("Surface")
                    .text_color(c.on_surface)
                    .modifier(
                        Modifier::new()
                            .fill_max_width()
                            .padding(8.0)
                            .background(c.surface),
                    )
                    .render(ui, dispatch);
                Text::new("Error")
                    .text_color(c.on_error)
                    .modifier(
                        Modifier::new()
                            .fill_max_width()
                            .padding(8.0)
                            .background(c.error),
                    )
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // Кнопка переключения темы
                Text::new("Переключение темы использует RootMsg::ToggleTheme:")
                    .render(ui, dispatch);
                Button::new("Переключить тему")
                    .on_click(RootMsg::ToggleTheme)
                    .colors(c.primary, c.primary)
                    .text_color(c.on_primary)
                    .modifier(Modifier::new().fill_max_width().padding(8.0))
                    .render(ui, dispatch);

                Text::new("Кнопка с белым текстом (не меняется от темы):").render(ui, dispatch);
                Button::new("Белый текст всегда")
                    .on_click(RootMsg::ToggleTheme)
                    .colors(
                        egui::Color32::from_rgb(0, 128, 255),
                        egui::Color32::from_rgb(255, 120, 0),
                    )
                    .text_color(egui::Color32::WHITE)
                    .modifier(Modifier::new().fill_max_width().padding(8.0))
                    .render(ui, dispatch);

                Spacer::new(16.0).render(ui, dispatch);
                Button::new("← Назад")
                    .on_click(RootMsg::Back)
                    .colors(c.primary, c.primary)
                    .text_color(c.on_primary)
                    .modifier(Modifier::new().fill_max_width().padding(8.0))
                    .render(ui, dispatch);
            });
    }
}
