//! WidgetsScreen — демонстрация базовых виджетов (Text, Button, Spacer, Icon).

use egui_android_framework::{
    runtime::Dispatcher,
    ui::{
        containers::Column,
        modifier::ModifierExt,
        widgets::{Button, Spacer, Text, Widget},
    },
};

use crate::root_component::RootMsg;

/// Экран демонстрации виджетов.
pub struct WidgetsScreen;

impl WidgetsScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<RootMsg>) {
        Column::new(ui, dispatch, |ui, dispatch| {
            Text::new("Виджеты").padding(8.0).render(ui, dispatch);
            Spacer::new(8.0).render(ui, dispatch);
            Text::new("Обычный текст").render(ui, dispatch);
            Spacer::new(4.0).render(ui, dispatch);
            Text::new("Кнопка:").render(ui, dispatch);
            Button::new("Нажми меня")
                .on_click(RootMsg::Back)
                .padding(8.0)
                .render(ui, dispatch);
            Spacer::new(8.0).render(ui, dispatch);
            Text::new("Spacer 16px:").render(ui, dispatch);
            Spacer::new(16.0).render(ui, dispatch);
            Text::new("Текст после Spacer").render(ui, dispatch);
            Spacer::new(16.0).render(ui, dispatch);
            Button::new("← Назад")
                .on_click(RootMsg::Back)
                .padding(8.0)
                .render(ui, dispatch);
        });
    }
}
