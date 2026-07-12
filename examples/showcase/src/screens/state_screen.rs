//! StateScreen — демонстрация локального состояния через remember.
//!
//! Показывает использование remember внутри замыканий Column
//! с кнопками on_click_with для модификации локального состояния.

use egui_android_framework::{
    runtime::Dispatcher,
    ui::{
        containers::Column,
        modifier::{Modifier, ModifierDsl},
        remember,
        theme::Theme,
        widgets::{Button, Spacer, Text, Widget},
        UiWrapper,
    },
};

use crate::root_component::RootMsg;

/// Экран демонстрации локального состояния.
pub struct StateScreen;

impl StateScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<RootMsg>) {
        let c = &Theme::current_from_ui(ui).colors;

        Column::new()
            .scrollable()
            .show(ui, dispatch, |ui, dispatch| {
                Text::new("Локальное состояние (remember)")
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);
                Spacer::new(8.0).render(ui, dispatch);

                // ──────────────────────────────────────
                // remember + on_click_with — интерактив
                // ──────────────────────────────────────
                Text::new("Счётчик через on_click_with:").render(ui, dispatch);

                let count = remember(ui, "ss_count", || 0i32);

                Text::new(format!("Значение: {}", count.get()))
                    .text_color(c.on_secondary)
                    .modifier(Modifier::new().padding(12.0).background(c.secondary))
                    .render(ui, dispatch);

                // Кнопки для изменения remember напрямую через on_click_with
                Button::new("+1")
                    .on_click_with({
                        let count = count.clone();
                        move |_ui, _dispatch| {
                            count.modify(|c| *c += 1);
                        }
                    })
                    .colors(c.secondary, c.secondary)
                    .text_color(c.on_secondary)
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);

                Button::new("-1")
                    .on_click_with({
                        let count = count.clone();
                        move |_ui, _dispatch| {
                            count.modify(|c| *c -= 1);
                        }
                    })
                    .colors(c.secondary, c.secondary)
                    .text_color(c.on_secondary)
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);

                Button::new("Сброс")
                    .on_click_with({
                        let count = count.clone();
                        move |_ui, _dispatch| {
                            count.set(0);
                        }
                    })
                    .colors(c.secondary, c.secondary)
                    .text_color(c.on_secondary)
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // ──────────────────────────────────────
                // remember + AnimatedVisibility через on_click_with
                // ──────────────────────────────────────
                Text::new("Анимация через on_click_with:").render(ui, dispatch);

                let expanded = remember(ui, "ss_expanded", || false);

                Text::new(format!(
                    "Состояние: {}",
                    if *expanded.get() {
                        "развёрнуто"
                    } else {
                        "свёрнуто"
                    }
                ))
                .text_color(c.on_secondary)
                .modifier(Modifier::new().padding(8.0).background(c.secondary))
                .render(ui, dispatch);

                Button::new(if *expanded.get() {
                    "Свернуть ▲"
                } else {
                    "Развернуть ▼"
                })
                .on_click_with({
                    let expanded = expanded.clone();
                    move |_ui, _dispatch| {
                        expanded.modify(|v| *v = !*v);
                    }
                })
                .colors(c.secondary, c.secondary)
                .text_color(c.on_secondary)
                .modifier(Modifier::new().padding(8.0))
                .render(ui, dispatch);

                // Комбинированный пример: on_click(msg) + on_click_with(closure)
                Spacer::new(16.0).render(ui, dispatch);
                Button::new("← Назад (MVI + remember)")
                    .on_click(RootMsg::Back)
                    .on_click_with({
                        let count = count.clone();
                        move |_ui, _dispatch| {
                            count.set(0);
                            log::info!("Счётчик сброшен при навигации назад");
                        }
                    })
                    .colors(c.primary, c.primary)
                    .text_color(c.on_primary)
                    .modifier(Modifier::new().fill_max_width().padding(8.0))
                    .render(ui, dispatch);

                Spacer::new(4.0).render(ui, dispatch);
                Text::new("Кнопка выше использует и on_click, и on_click_with вместе")
                    .modifier(Modifier::new().padding(4.0))
                    .render(ui, dispatch);
            });
    }
}
