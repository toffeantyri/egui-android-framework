//! View-функция счётчика.
//!
//! Чистая функция от состояния — не хранит состояние, не знает о каналах.
//! Сообщения отправляются через `Dispatcher` в момент события.
//! Использует контейнеры, виджеты и модификаторы.

use egui_android_framework::{
    containers::Column,
    modifiers::ModifierExt,
    widgets::{Button, Spacer, Text, Widget},
    Dispatcher,
};

use crate::msg::Msg;

/// View-функция счётчика: читает состояние, рисует UI через виджеты с модификаторами.
pub fn counter_view(state: &u32, ui: &mut egui::Ui, dispatch: &Dispatcher<Msg>) {
    ui.vertical_centered(|ui| {
        Column::empty()
            .child(Spacer::new(60.0))
            .child(Text::new("egui Counter (v2)"))
            .child(Spacer::new(16.0))
            .child(
                Text::new(format!("{}", state))
                    .padding(16.0)
                    .background(egui::Color32::from_gray(40)),
            )
            .child(Spacer::new(24.0))
            .child(
                Button::new("+1")
                    .on_click(Msg::Increment)
                    .padding(8.0)
                    .background(egui::Color32::from_rgb(0, 128, 255)),
            )
            .render(ui, dispatch);
    });
}
