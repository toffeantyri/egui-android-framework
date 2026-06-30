//! ModifiersScreen — демонстрация модификаторов.

use egui_android_framework::{
    dispatcher::Dispatcher,
    modifiers::ModifierExt,
    remember,
    widgets::{Button, Spacer, Text, Widget},
};

use crate::root_component::RootMsg;

/// Экран демонстрации модификаторов.
pub struct ModifiersScreen;

impl ModifiersScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, ui: &mut egui::Ui, _dispatch: &Dispatcher<RootMsg>) {
        let mut click_count = remember(ui, "mod_click_count", || 0i32);

        Text::new("Модификаторы").render(ui, _dispatch);
        Spacer::new(8.0).render(ui, _dispatch);

        Text::new("Padding 8px:").render(ui, _dispatch);
        Text::new("Текст с padding")
            .padding(8.0)
            .background(egui::Color32::from_gray(60))
            .render(ui, _dispatch);

        Spacer::new(8.0).render(ui, _dispatch);
        Text::new("Background:").render(ui, _dispatch);
        Text::new("Синий фон")
            .background(egui::Color32::from_rgb(0, 80, 200))
            .padding(12.0)
            .render(ui, _dispatch);

        Spacer::new(8.0).render(ui, _dispatch);
        Text::new("Size + Background:").render(ui, _dispatch);
        Text::new("200x48")
            .size(200.0, 48.0)
            .background(egui::Color32::from_gray(50))
            .render(ui, _dispatch);

        Spacer::new(8.0).render(ui, _dispatch);
        Text::new("Clickable:").render(ui, _dispatch);
        if ui
            .button(format!("Нажатий: {}", *click_count.get()))
            .clicked()
        {
            click_count.modify(|c| *c += 1);
        }

        Spacer::new(16.0).render(ui, _dispatch);
        Button::new("Назад")
            .on_click(RootMsg::Back)
            .render(ui, _dispatch);
    }
}
