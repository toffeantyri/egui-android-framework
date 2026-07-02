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
        Column::<RootMsg>::empty()
            .child(Spacer::new(16.0))
            .child(Text::new("Виджеты").padding(8.0))
            .child(Spacer::new(8.0))
            .child(Text::new("Обычный текст"))
            .child(Spacer::new(4.0))
            .child(Text::new("Кнопка:"))
            .child(
                Button::new("Нажми меня")
                    .on_click(RootMsg::Back)
                    .padding(8.0),
            )
            .child(Spacer::new(8.0))
            .child(Text::new("Spacer 16px:"))
            .child(Spacer::new(16.0))
            .child(Text::new("Текст после Spacer"))
            .child(Spacer::new(16.0))
            .child(Button::new("← Назад").on_click(RootMsg::Back).padding(8.0))
            .render(ui, dispatch);
    }
}
