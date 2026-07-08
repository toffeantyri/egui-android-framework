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
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                Text::new("Кнопка с кастомными цветами:").render(ui, dispatch);
                Button::new("Зелёная")
                    .colors(
                        egui::Color32::from_rgb(0, 160, 80),  // обычный
                        egui::Color32::from_rgb(0, 255, 130), // нажатый
                    )
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);
                Text::new("Spacer 16px:").render(ui, dispatch);
                Spacer::new(16.0)
                    .modifier(Modifier::new().border(1.0_f32, egui::Color32::from_rgb(0, 255, 130)))
                    .render(ui, dispatch);
                Text::new("Текст после Spacer").render(ui, dispatch);
                Spacer::new(16.0).render(ui, dispatch);
                Button::new("← Назад")
                    .on_click(RootMsg::Back)
                                        .modifier(Modifier::new().fill_max_width().padding(8.0))
                                        .render(ui, dispatch);
                                });
    }
}
