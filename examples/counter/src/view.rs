//! View-функция счётчика.
//!
//! Чистая функция от состояния — не хранит состояние, не знает о каналах.
//! Сообщения отправляются через `Dispatcher` в момент события.
//!
//! Использует Compose-like API из крейтов egui-android-framework:
//! Column, Button, Text, Spacer, AnimatedVisibility, AnimationExt, ModifierExt.

use egui_android_framework::core::UiWrapper;
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
pub fn counter_view(state: &u32, ui: &mut UiWrapper, dispatch: &Dispatcher<Msg>) {
    // Весь UI строится через наш фреймворк, без нативного egui
    Column::new()
        .scrollable()
        .show(ui, dispatch, |ui, dispatch| {
            // remember внутри замыкания (работает благодаря Arc<RwLock<T>>)
            let show_details = remember(ui, "show_details", || false);

            Text::new("egui Counter (v2)").render(ui, dispatch);
            Spacer::new(16.0).render(ui, dispatch);

            // Поле счётчика на всю ширину, текст по центру
            Text::new(format!("{}", state))
                .font_size(24.0)
                .padding(30.0)
                .align(egui::Align::Center)
                .size(ui.available_width(), 48.0)
                .background(egui::Color32::from_gray(40))
                .render(ui, dispatch);

            Spacer::new(8.0).render(ui, dispatch);
            Button::new("+1")
                .on_click(Msg::Increment)
                .padding(8.0)
                .background(egui::Color32::from_rgb(0, 128, 255))
                .render(ui, dispatch);

            // Toggle details — через on_click_with, без MVI-потока
            Spacer::new(8.0).render(ui, dispatch);
            Button::new(if *show_details.get() {
                "Скрыть details"
            } else {
                "Показать details"
            })
            .on_click_with({
                let show_details = show_details.clone();
                move |_ui, _dispatch| {
                    show_details.modify(|v| *v = !*v);
                }
            })
            .padding(8.0)
            .render(ui, dispatch);

            // Анимированное появление
            AnimatedVisibility::<Msg>::new(*show_details.get(), 0.4)
                .child(
                    Text::new("Анимированное появление!")
                        .padding(12.0)
                        .background(egui::Color32::from_gray(50)),
                )
                .render(ui, dispatch);

            // Fade-текст через AnimationExt
            Spacer::new(8.0).render(ui, dispatch);
            Text::new("Fade-текст (прозрачность 0.5)")
                .fade(0.5)
                .render(ui, dispatch);
        });
}
