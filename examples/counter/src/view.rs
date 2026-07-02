//! View-функция счётчика.
//!
//! Чистая функция от состояния — не хранит состояние, не знает о каналах.
//! Сообщения отправляются через `Dispatcher` в момент события.
//!
//! Использует Compose-like API из крейтов egui-android-framework:
//! Column, Button, Text, Spacer, AnimatedVisibility, AnimationExt, ModifierExt.

use egui_android_framework::runtime::Dispatcher;
use egui_android_framework::ui::{
    animation::{AnimatedVisibility, AnimationExt},
    containers::Column,
    modifier::ModifierExt,
    remember,
    widgets::{Button, Spacer, Text, Widget},
};

use crate::msg::Msg;

/// View-функция счётчика: читает состояние, рисует UI.
pub fn counter_view(state: &u32, ui: &mut egui::Ui, dispatch: &Dispatcher<Msg>) {
    // ── Вертикальная колонка с виджетами ──────────────────────────
    Column::new(ui, dispatch, |ui, dispatch| {
        // remember ВНУТРИ замыкания (работает благодаря RwLock)
        let show_details = remember(ui, "show_details", || false);

        Text::new("egui Counter (v2)").render(ui, dispatch);
        Spacer::new(16.0).render(ui, dispatch);
        Text::new(format!("{}", state))
            .padding(16.0)
            .background(egui::Color32::from_gray(40))
            .render(ui, dispatch);
        Button::new("+1")
            .on_click(Msg::Increment)
            .padding(8.0)
            .background(egui::Color32::from_rgb(0, 128, 255))
            .render(ui, dispatch);

        // Toggle details — кнопка on_click отправляет Msg, а не замыкание
        Button::new(if *show_details.get() {
            "Скрыть details"
        } else {
            "Показать details"
        })
        .on_click(Msg::ToggleDetails)
        .padding(8.0)
        .render(ui, dispatch);

        // Анимированное появление (внутри Column)
        AnimatedVisibility::<Msg>::new(*show_details.get(), 0.4)
            .child(
                Text::new("Анимированное появление!")
                    .padding(12.0)
                    .background(egui::Color32::from_gray(50)),
            )
            .render(ui, dispatch);

        // Fade-текст через AnimationExt
        Text::new("Fade-текст (прозрачность 0.5)")
            .fade(0.5)
            .render(ui, dispatch);
    });
}
