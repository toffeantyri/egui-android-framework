//! StateScreen — демонстрация локального состояния через remember.
//!
//! Показывает использование remember внутри замыканий Column
//! с кнопками on_click_with для модификации локального состояния.

use egui_android_framework::{
    runtime::Dispatcher,
    ui::{
        containers::Column,
        modifier::ModifierExt,
        remember,
        widgets::{Button, Spacer, Text, Widget},
    },
};

use crate::root_component::RootMsg;

/// Экран демонстрации локального состояния.
pub struct StateScreen;

impl StateScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<RootMsg>) {
        Column::new(ui, dispatch, |ui, dispatch| {
            Text::new("Локальное состояние (remember)")
                .padding(8.0)
                .render(ui, dispatch);
            Spacer::new(8.0).render(ui, dispatch);

            // ──────────────────────────────────────
            // remember + on_click_with — интерактив
            // ──────────────────────────────────────
            Text::new("Счётчик через on_click_with:").render(ui, dispatch);

            let count = remember(ui, "ss_count", || 0i32);

            Text::new(format!("Значение: {}", count.get()))
                .padding(12.0)
                .background(egui::Color32::from_gray(60))
                .render(ui, dispatch);

            // Кнопки для изменения remember напрямую через on_click_with
            Button::new("+1")
                .on_click_with({
                    let count = count.clone();
                    move |_ui, _dispatch| {
                        count.modify(|c| *c += 1);
                    }
                })
                .padding(8.0)
                .render(ui, dispatch);

            Button::new("-1")
                .on_click_with({
                    let count = count.clone();
                    move |_ui, _dispatch| {
                        count.modify(|c| *c -= 1);
                    }
                })
                .padding(8.0)
                .render(ui, dispatch);

            Button::new("Сброс")
                .on_click_with({
                    let count = count.clone();
                    move |_ui, _dispatch| {
                        count.set(0);
                    }
                })
                .padding(8.0)
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
            .padding(8.0)
            .background(egui::Color32::from_gray(50))
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
            .padding(8.0)
            .render(ui, dispatch);

            // Комбинированный пример: on_click(msg) + on_click_with(closure)
            // on_click диспатчит RootMsg::Back для навигации
            Spacer::new(16.0).render(ui, dispatch);
            Button::new("← Назад (MVI + remember)")
                .on_click(RootMsg::Back) // MVI-поток: навигация назад
                .on_click_with({
                    let count = count.clone();
                    move |_ui, _dispatch| {
                        // Дополнительно: сбрасываем счётчик при уходе
                        count.set(0);
                        log::info!("Счётчик сброшен при навигации назад");
                    }
                })
                .padding(8.0)
                .render(ui, dispatch);

            Spacer::new(4.0).render(ui, dispatch);
            Text::new("Кнопка выше использует и on_click, и on_click_with вместе")
                .padding(4.0)
                .render(ui, dispatch);
        });
    }
}
