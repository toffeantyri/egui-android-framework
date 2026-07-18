//! View-функция счётчика.
//!
//! Чистая функция от состояния — не хранит состояние, не знает о каналах.
//! Сообщения отправляются через `Dispatcher` в момент события.
//!
//! Использует Compose-like API из крейтов egui-android-framework:
//! Column, Button, Text, Spacer, AnimatedVisibility, AnimationExt.
//!
//! Все цвета соответствуют Material Design 3 теме (Theme::current).
//! - background / on_background — фон страницы и текст на нём.
//! - secondary / on_secondary — для карточек и поверхностей.
//! - primary / on_primary — для акцентных кнопок.

use egui_android_core::UiWrapper;
use egui_android_runtime::Dispatcher;
use egui_android_ui::{
    animation::{AnimatedVisibility, AnimationExt},
    containers::Column,
    modifier::{Modifier, ModifierDsl},
    remember,
    theme::Theme,
    widgets::{Button, Spacer, Text, Widget},
};

use crate::msg::Msg;

/// View-функция счётчика: читает состояние, рисует UI.
pub fn counter_view(state: &u32, ui: &mut UiWrapper, dispatch: &Dispatcher<Msg>) {
    // Получаем текущие цвета темы
    let theme = Theme::current_from_ui(ui);
    let c = &theme.colors;

    Column::new()
        .scrollable()
        .show(ui, dispatch, |ui, dispatch| {
            let show_details = remember(ui, "show_details", || false);

            // ── Заголовок (центрированный, secondary фон) ──────────────
            Text::new("*** egui compose counter ***")
                .font_size(20.0)
                .align(egui::Align::Center)
                .text_color(c.on_secondary)
                .modifier(
                    Modifier::new()
                        .fill_max_width()
                        .padding(16.0)
                        .background(c.secondary),
                )
                .render(ui, dispatch);

            Spacer::new(16.0).render(ui, dispatch);

            // ── Значение счётчика (центрированный, secondary фон) ──────
            Text::new(format!("{}", state))
                .font_size(24.0)
                .align(egui::Align::RIGHT)
                .text_color(c.on_secondary)
                .modifier(
                    Modifier::new()
                        .fill_max_width()
                        .padding(16.0)
                        .height(48.0)
                        .background(c.secondary),
                )
                .render(ui, dispatch);

            Spacer::new(8.0).render(ui, dispatch);

            // ── Кнопка +1 (primary цвета) ──────────────────────────────
            Button::new("+1")
                .on_click(Msg::Increment)
                .theme_colors(c.primary)
                .text_color(c.on_primary)
                .modifier(Modifier::new().fill_max_width().padding(8.0))
                .render(ui, dispatch);

            Spacer::new(8.0).render(ui, dispatch);

            // ── Toggle details (secondary цвета) ───────────────────────
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
            .theme_colors(c.secondary)
            .text_color(c.on_secondary)
            .modifier(Modifier::new().fill_max_width().padding(8.0))
            .render(ui, dispatch);

            // ── Переключение темы (primary цвета) ──────────────────────
            Spacer::new(8.0).render(ui, dispatch);
            Button::new("Переключить тему")
                .on_click(Msg::ToggleTheme)
                .theme_colors(c.primary)
                .text_color(c.on_primary)
                .modifier(Modifier::new().fill_max_width().padding(8.0))
                .render(ui, dispatch);

            Spacer::new(8.0).render(ui, dispatch);

            // ── Анимированное появление ────────────────────────────────
            AnimatedVisibility::<Msg>::new(*show_details.get(), 0.4)
                .child(
                    Text::new("Анимированное появление!")
                        .align(egui::Align::Center)
                        .text_color(c.on_secondary)
                        .modifier(
                            Modifier::new()
                                .fill_max_width()
                                .padding(12.0)
                                .background(c.secondary),
                        ),
                )
                .render(ui, dispatch);

            Spacer::new(8.0).render(ui, dispatch);

            // ── Fade-текст ─────────────────────────────────────────────
            Text::new("Fade-текст (прозрачность 0.5)")
                .text_color(c.on_secondary)
                .fade(0.5)
                .modifier(
                    Modifier::new()
                        .fill_max_width()
                        .padding(8.0)
                        .background(c.secondary),
                )
                .render(ui, dispatch);
        });
}
