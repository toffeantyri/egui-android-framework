//! StateScreen — демонстрация локального состояния через remember.

use egui_android_framework::{
    runtime::Dispatcher,
    ui::{
        animation::AnimatedVisibility,
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
        let mut expanded = remember(ui, "ss_expanded", || false);
        let mut local_count = remember(ui, "ss_local_count", || 0i32);

        Column::new(ui, dispatch, |ui, dispatch| {
            Text::new("Локальное состояние (remember)")
                .padding(8.0)
                .render(ui, dispatch);
            Spacer::new(8.0).render(ui, dispatch);

            // Счётчик
            Text::new("Счётчик (локальный):").render(ui, dispatch);
            Text::new(format!("{}", *local_count.get()))
                .padding(16.0)
                .background(egui::Color32::from_gray(60))
                .render(ui, dispatch);

            Spacer::new(8.0).render(ui, dispatch);

            // Аккордеон
            Text::new("Аккордеон (remember):").render(ui, dispatch);
            AnimatedVisibility::new(*expanded.get(), 0.2)
                .child(
                    Text::new("Этот контент управляется remember.")
                        .padding(12.0)
                        .background(egui::Color32::from_gray(50)),
                )
                .render(ui, dispatch);

            Spacer::new(16.0).render(ui, dispatch);
            Button::new("← Назад")
                .on_click(RootMsg::Back)
                .padding(8.0)
                .render(ui, dispatch);
        });

        // Кнопки управления remember (вне Column — RememberState требует &mut)
        if ui.button("+").clicked() {
            local_count.modify(|c| *c += 1);
        }
        if ui.button("-").clicked() {
            local_count.modify(|c| *c -= 1);
        }
        if ui
            .button(if *expanded.get() {
                "Свернуть ▲"
            } else {
                "Развернуть ▼"
            })
            .clicked()
        {
            expanded.modify(|v| *v = !*v);
        }
    }
}
