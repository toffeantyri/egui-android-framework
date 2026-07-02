//! ModifiersScreen — демонстрация модификаторов.

use egui::Frame;
use egui_android_framework::runtime::Dispatcher;
use egui_android_framework::ui::remember;

use crate::root_component::RootMsg;

/// Экран демонстрации модификаторов.
pub struct ModifiersScreen;

impl ModifiersScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<RootMsg>) {
        let mut click_count = remember(ui, "mod_click_count", || 0i32);

        ui.heading("Модификаторы");
        ui.add_space(8.0);

        ui.label("Padding 8px:");
        Frame::new()
            .fill(egui::Color32::from_gray(60))
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.label("Текст с padding");
            });

        ui.add_space(8.0);
        ui.label("Background:");
        Frame::new()
            .fill(egui::Color32::from_rgb(0, 80, 200))
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.label("Синий фон");
            });

        ui.add_space(8.0);
        ui.label("Size + Background:");
        Frame::new()
            .fill(egui::Color32::from_gray(50))
            .inner_margin(12.0)
            .show(ui, |ui| {
                ui.label("200x48 (fixed size not available)");
            });

        ui.add_space(8.0);
        ui.label("Clickable:");
        if ui
            .button(format!("Нажатий: {}", *click_count.get()))
            .clicked()
        {
            click_count.modify(|c| *c += 1);
        }

        ui.add_space(16.0);
        if ui.button("Назад").clicked() {
            dispatch.dispatch(RootMsg::Back);
        }
    }
}
