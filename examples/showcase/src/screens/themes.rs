//! ThemesScreen — демонстрация тем (light/dark).

use egui::Frame;
use egui_android_framework::runtime::Dispatcher;

use crate::root_component::RootMsg;

/// Экран демонстрации тем.
pub struct ThemesScreen {
    pub is_dark_mode: bool,
}

impl ThemesScreen {
    pub fn new() -> Self {
        Self {
            is_dark_mode: false,
        }
    }

    pub fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<RootMsg>) {
        let visuals = ui.visuals().clone();

        ui.heading("Темы");
        ui.add_space(8.0);

        ui.label("Текущая тема:");
        Frame::new()
            .fill(visuals.selection.bg_fill)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.label(if self.is_dark_mode {
                    "Тёмная"
                } else {
                    "Светлая"
                });
            });

        ui.add_space(8.0);
        ui.label("Палитра цветов:");

        let colors = [
            ("Widgets bg", visuals.widgets.noninteractive.bg_fill),
            ("Faint bg", visuals.window_fill()),
            ("Panel fill", visuals.panel_fill),
            ("Hyperlink", visuals.hyperlink_color),
            ("Selection", visuals.selection.bg_fill),
        ];

        for (name, color) in &colors {
            Frame::new().fill(*color).inner_margin(8.0).show(ui, |ui| {
                ui.label(*name);
            });
        }

        ui.add_space(8.0);
        if ui.button("Переключить тему").clicked() {
            dispatch.dispatch(RootMsg::ToggleTheme);
        }

        ui.add_space(16.0);
        if ui.button("Назад").clicked() {
            dispatch.dispatch(RootMsg::Back);
        }
    }
}
