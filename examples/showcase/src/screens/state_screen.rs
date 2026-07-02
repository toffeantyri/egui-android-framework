//! StateScreen — демонстрация локального состояния через remember.

use egui::Frame;
use egui_android_framework::runtime::Dispatcher;
use egui_android_framework::ui::remember;

use crate::root_component::RootMsg;

/// Экран демонстрации локального состояния.
pub struct StateScreen;

impl StateScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<RootMsg>) {
        let mut expanded = remember(ui, "ss_expanded", || false);
        let mut local_count = remember(ui, "ss_local_count", || 0i32);

        ui.heading("Локальное состояние (remember)");
        ui.add_space(8.0);

        ui.label("Счётчик (локальный):");
        Frame::new()
            .fill(egui::Color32::from_gray(60))
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.label(format!("{}", *local_count.get()));
            });

        if ui.button("+").clicked() {
            local_count.modify(|c| *c += 1);
        }
        if ui.button("-").clicked() {
            local_count.modify(|c| *c -= 1);
        }

        ui.add_space(8.0);
        ui.label("Аккордеон (remember):");
        if ui
            .button(if *expanded.get() {
                "Свернуть ▲"
            } else {
                "Развернуть ▼"
            })
            .clicked()
        {
            expanded.modify(|v| *v = !*v);
        }

        if *expanded.get() {
            Frame::new()
                .fill(egui::Color32::from_gray(50))
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.label("Этот контент управляется remember.");
                });
        }

        ui.add_space(16.0);
        if ui.button("Назад").clicked() {
            dispatch.dispatch(RootMsg::Back);
        }
    }
}
