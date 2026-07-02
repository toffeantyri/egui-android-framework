//! WidgetsScreen — демонстрация базовых виджетов.

use egui_android_framework::runtime::Dispatcher;

use crate::root_component::RootMsg;

/// Экран демонстрации виджетов.
pub struct WidgetsScreen;

impl WidgetsScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<RootMsg>) {
        ui.heading("Виджеты");
        ui.add_space(8.0);
        ui.label("Обычный текст");
        ui.add_space(4.0);
        ui.label("Кнопка:");
        if ui.button("Нажми меня").clicked() {}
        ui.add_space(8.0);
        ui.label("Spacer 16px:");
        ui.add_space(16.0);
        ui.label("Текст после Spacer");

        ui.add_space(16.0);
        if ui.button("Назад").clicked() {
            dispatch.dispatch(RootMsg::Back);
        }
    }
}
