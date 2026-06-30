//! View-функция счётчика.
//!
//! Чистая функция от состояния — не хранит состояние, не знает о каналах.
//! Сообщения отправляются через `Dispatcher` в момент события.
//! Использует контейнеры, виджеты, модификаторы, remember и анимации.

use egui_android_framework::{
    animation::{AnimatedVisibility, AnimationExt},
    containers::Column,
    modifiers::ModifierExt,
    remember,
    widgets::{Button, Spacer, Text, Widget},
    Dispatcher,
};

use crate::msg::Msg;

/// View-функция счётчика: читает состояние, рисует UI через виджеты с модификаторами.
pub fn counter_view(state: &u32, ui: &mut egui::Ui, dispatch: &Dispatcher<Msg>) {
    // Локальное состояние — показывать ли подробности
    let mut show_details = remember(ui, "show_details", || false);

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

        // Переключатель локального состояния
        if ui.button("Toggle details").clicked() {
            show_details.modify(|v| *v = !*v);
        }

        // Анимированное появление/исчезновение с Fade
        AnimatedVisibility::new(*show_details.get(), 0.4)
            .child(
                Text::new("Анимированное появление!")
                    .padding(12.0)
                    .background(egui::Color32::from_gray(50)),
            )
            .render(ui, dispatch);

        // Полупрозрачный текст через Fade
        Text::new("Fade-текст (прозрачность 0.5)")
            .fade(0.5)
            .render(ui, dispatch);
    });
}
