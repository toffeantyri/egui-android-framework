//! WidgetsScreen — демонстрация базовых виджетов (Text, Button, Spacer, Icon).
//!
//! Демонстрирует:
//! - Text, Spacer — базовые виджеты для отображения
//! - Button с визуальной обратной связью при нажатии (встроенная в фреймворк)
//! - кастомные цвета кнопки через Button::colors()

use egui_android_framework::{
    runtime::Dispatcher,
    ui::{
        containers::Column,
        modifier::{Modifier, ModifierDsl},
        theme::Theme,
        widgets::{Button, Spacer, Text, Widget},
        UiWrapper,
    },
};

use crate::root_component::RootMsg;

/// Экран демонстрации виджетов.
pub struct WidgetsScreen;

impl WidgetsScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<RootMsg>) {
        let c = &Theme::current_from_ui(ui).colors;

        Column::new()
            .scrollable()
            .show(ui, dispatch, |ui, dispatch| {
                Text::new("Виджеты")
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);
                Spacer::new(8.0).render(ui, dispatch);
                Text::new("Обычный текст").render(ui, dispatch);
                Spacer::new(4.0).render(ui, dispatch);
                Text::new("Кнопка с реакцией на нажатие:").render(ui, dispatch);

                // Стандартный Button — цвет меняется при нажатии (встроено)
                Button::new("Нажми меня")
                    .colors(c.primary, c.primary)
                    .text_color(c.on_primary)
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                Text::new("Кнопка с кастомными цветами:").render(ui, dispatch);
                Button::new("Вторичная")
                    .colors(c.secondary, c.secondary)
                    .text_color(c.on_secondary)
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);
                Text::new("Spacer 16px:").render(ui, dispatch);
                Spacer::new(16.0)
                    .modifier(Modifier::new().border(1.0, c.outline))
                    .render(ui, dispatch);
                Text::new("Текст после Spacer").render(ui, dispatch);
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
