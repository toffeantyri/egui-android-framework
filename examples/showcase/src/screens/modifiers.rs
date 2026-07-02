//! ModifiersScreen — демонстрация модификаторов padding, size, background, align, clickable.

use egui_android_framework::{
    runtime::Dispatcher,
    ui::{
        containers::Column,
        modifier::ModifierExt,
        remember,
        widgets::{Button, Spacer, Text, Widget},
    },
};

use crate::root_component::RootMsg;

/// Экран демонстрации модификаторов.
pub struct ModifiersScreen;

impl ModifiersScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<RootMsg>) {
        let mut click_count = remember(ui, "mod_click_count", || 0i32);

        Column::<RootMsg>::empty()
            .child(Spacer::new(16.0))
            .child(Text::new("Модификаторы").padding(8.0))
            .child(Spacer::new(8.0))
            // Padding
            .child(Text::new("Padding 8px:"))
            .child(
                Text::new("Текст с padding")
                    .padding(8.0)
                    .background(egui::Color32::from_gray(60)),
            )
            .child(Spacer::new(8.0))
            // Background
            .child(Text::new("Background:"))
            .child(
                Text::new("Синий фон")
                    .background(egui::Color32::from_rgb(0, 80, 200))
                    .padding(12.0),
            )
            .child(Spacer::new(8.0))
            // Size + Background
            .child(Text::new("Size + Background:"))
            .child(
                Text::new("200x48")
                    .size(200.0, 48.0)
                    .background(egui::Color32::from_gray(50)),
            )
            .child(Spacer::new(8.0))
            // Clickable через модификатор (диспатчит RootMsg::Back)
            .child(Text::new("Clickable (нажми на текст):"))
            .child(
                Text::new("Нажми меня — назад")
                    .padding(8.0)
                    .background(egui::Color32::from_gray(60))
                    .clickable(RootMsg::Back),
            )
            .child(Spacer::new(8.0))
            // Счётчик через remember
            .child(Text::new("Счётчик (remember):"))
            .child(
                Text::new(format!("Значение: {}", *click_count.get()))
                    .padding(8.0)
                    .background(egui::Color32::from_gray(60)),
            )
            .child(Spacer::new(16.0))
            .child(Button::new("← Назад").on_click(RootMsg::Back).padding(8.0))
            .render(ui, dispatch);

        // Кнопка для изменения локального remember-состояния
        // (не может быть внутри Column, так как RememberState требует прямого доступа)
        if ui.button("+1").clicked() {
            click_count.modify(|c| *c += 1);
        }
    }
}
