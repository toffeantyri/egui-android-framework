//! ModifiersScreen — демонстрация модификаторов padding, size, background, align, clickable.

use egui_android_framework::{
    runtime::Dispatcher,
    ui::{
        containers::Column,
        modifier::{Modifier, ModifierApply, ModifierExt},
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
        // remember ВНУТРИ замыкания (работает благодаря RwLock)
        let click_count = remember(ui, "mod_click_count", || 0i32);

        Column::new()
            .scrollable()
            .show(ui, dispatch, |ui, dispatch| {
                Text::new("Модификаторы").padding(8.0).render(ui, dispatch);
                Spacer::new(8.0).render(ui, dispatch);

                // Padding
                Text::new("Padding 8px:").render(ui, dispatch);
                Text::new("Текст с padding")
                    .padding(8.0)
                    .background(egui::Color32::from_gray(60))
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // Background
                Text::new("Background:").render(ui, dispatch);
                Text::new("Синий фон")
                    .background(egui::Color32::from_rgb(0, 80, 200))
                    .padding(12.0)
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // Size + Background
                Text::new("Size + Background:").render(ui, dispatch);
                Text::new("200x48")
                    .size(200.0, 48.0)
                    .background(egui::Color32::from_gray(50))
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // Clickable через модификатор (диспатчит RootMsg::Back)
                Text::new("Clickable (нажми на текст):").render(ui, dispatch);
                Text::new("Нажми меня — назад")
                    .padding(8.0)
                    .background(egui::Color32::from_gray(60))
                    .clickable(RootMsg::Back)
                    .render(ui, dispatch);

                // Счётчик через remember внутри замыкания
                Spacer::new(8.0).render(ui, dispatch);
                Text::new("Счётчик (remember внутри замыкания):").render(ui, dispatch);
                Text::new(format!("Значение: {}", click_count.get()))
                    .padding(8.0)
                    .background(egui::Color32::from_gray(60))
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);
                Button::new("← Назад")
                    .on_click(RootMsg::Back)
                    .modifier(Modifier::new().fill_max_width().padding(8.0))
                    .render(ui, dispatch);
            });
    }
}
