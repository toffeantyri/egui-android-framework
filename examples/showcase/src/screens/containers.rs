//! ContainersScreen — демонстрация контейнеров.

use egui::Frame;
use egui_android_framework::runtime::Dispatcher;

use crate::root_component::RootMsg;

/// Экран демонстрации контейнеров.
pub struct ContainersScreen;

impl ContainersScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<RootMsg>) {
        ui.heading("Контейнеры");
        ui.add_space(8.0);

        ui.label("Column (вертикально):");
        ui.vertical(|ui| {
            ui.colored_label(egui::Color32::from_gray(50), "A");
            ui.add_space(4.0);
            ui.colored_label(egui::Color32::from_gray(60), "B");
        });

        ui.add_space(8.0);
        ui.label("Row (горизонтально):");
        ui.horizontal(|ui| {
            ui.colored_label(egui::Color32::from_gray(50), "X");
            ui.add_space(8.0);
            ui.colored_label(egui::Color32::from_gray(60), "Y");
            ui.add_space(8.0);
            ui.colored_label(egui::Color32::from_gray(70), "Z");
        });

        ui.add_space(8.0);
        ui.label("Stack (наложение):");
        Frame::new().fill(egui::Color32::BLUE).show(ui, |ui| {
            ui.label("Фон");
        });
        ui.label("Поверх");

        ui.add_space(8.0);
        ui.label("LazyColumn (список):");
        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                for i in 1..=10 {
                    let resp = Frame::new()
                        .fill(egui::Color32::from_gray(40))
                        .inner_margin(4.0)
                        .show(ui, |ui| {
                            ui.label(format!("Элемент {}", i));
                        });
                    ui.add_space(2.0);
                    let _ = resp;
                }
            });

        ui.add_space(16.0);
        if ui.button("Назад").clicked() {
            dispatch.dispatch(RootMsg::Back);
        }
    }
}
