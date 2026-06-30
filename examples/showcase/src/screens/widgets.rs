//! WidgetsScreen — демонстрация базовых виджетов.

use egui_android_framework::{
    dispatcher::Dispatcher,
    widgets::{Button, Spacer, Text, Widget},
};

use crate::root_component::RootMsg;

/// Экран демонстрации виджетов.
pub struct WidgetsScreen;

impl WidgetsScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, ui: &mut egui::Ui, _dispatch: &Dispatcher<RootMsg>) {
        Text::new("Виджеты").render(ui, _dispatch);
        Spacer::new(8.0).render(ui, _dispatch);
        Text::new("Обычный текст").render(ui, _dispatch);
        Spacer::new(4.0).render(ui, _dispatch);
        Text::new("Кнопка:").render(ui, _dispatch);
        Button::new("Нажми меня").render(ui, _dispatch);
        Spacer::new(8.0).render(ui, _dispatch);
        Text::new("Spacer 16px:").render(ui, _dispatch);
        Spacer::new(16.0).render(ui, _dispatch);
        Text::new("Текст после Spacer").render(ui, _dispatch);

        Spacer::new(16.0).render(ui, _dispatch);
        if ui.button("Назад").clicked() {}
        Button::new("Назад")
            .on_click(RootMsg::Back)
            .render(ui, _dispatch);
    }
}
