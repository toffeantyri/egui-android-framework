//! View-функция счётчика.
//!
//! Чистая функция от состояния — не хранит состояние, не знает о каналах.
//! Сообщения отправляются через `Dispatcher` в момент события.

use egui_android_framework::Dispatcher;

use crate::msg::Msg;

/// View-функция счётчика: читает состояние, рисует UI, диспатчит сообщения.
pub fn counter_view(state: &u32, ui: &mut egui::Ui, dispatch: &Dispatcher<Msg>) {
    ui.vertical_centered(|ui| {
        ui.add_space(60.0);
        ui.heading("egui Counter (v2)");
        ui.add_space(16.0);
        ui.label(
            egui::RichText::new(format!("{}", state))
                .size(72.0)
                .color(egui::Color32::from_rgb(66, 133, 244)),
        );
        ui.add_space(24.0);

        if ui
            .add_sized([200.0, 56.0], egui::Button::new("+1"))
            .clicked()
        {
            log::info!("UI: +1 clicked");
            dispatch.dispatch(Msg::Increment);
        }
    });
}
