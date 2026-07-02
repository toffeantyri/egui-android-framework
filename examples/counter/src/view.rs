//! View-функция счётчика.
//!
//! Чистая функция от состояния — не хранит состояние, не знает о каналах.
//! Сообщения отправляются через `Dispatcher` в момент события.

use egui_android_framework::runtime::Dispatcher;
use egui_android_framework::ui::remember;

use crate::msg::Msg;

/// View-функция счётчика: читает состояние, рисует UI.
pub fn counter_view(state: &u32, ui: &mut egui::Ui, dispatch: &Dispatcher<Msg>) {
    // Локальное состояние — показывать ли подробности
    let mut show_details = remember(ui, "show_details", || false);

    ui.vertical_centered(|ui| {
        ui.add_space(60.0);
        ui.label("egui Counter (v2)");
        ui.add_space(16.0);

        // Отображаем значение счётчика с рамкой
        let response = ui.label(format!("{}", state));
        ui.add_space(24.0);

        // Кнопка "+1"
        if ui.button("+1").clicked() {
            dispatch.dispatch(Msg::Increment);
        }

        // Переключатель локального состояния
        if ui.button("Toggle details").clicked() {
            show_details.modify(|v| *v = !*v);
        }

        // Анимированное появление/исчезновение (используя egui animate_bool)
        let id = ui.id().with("animated_details");
        let progress = ui
            .ctx()
            .animate_bool_with_time(id, *show_details.get(), 0.4);

        if progress > 0.0 {
            ui.scope(|ui| {
                ui.multiply_opacity(progress);
                ui.label("Анимированное появление!");
            });
        }

        ui.label("Fade-текст (прозрачность 0.5)");
    });
}
