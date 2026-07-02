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

        Column::<RootMsg>::empty()
            .child(Spacer::new(16.0))
            .child(Text::new("Локальное состояние (remember)").padding(8.0))
            .child(Spacer::new(8.0))
            // Счётчик
            .child(Text::new("Счётчик (локальный):"))
            .child(
                Text::new(format!("{}", *local_count.get()))
                    .padding(16.0)
                    .background(egui::Color32::from_gray(60)),
            )
            .child(Spacer::new(8.0))
            // Аккордеон
            .child(Text::new("Аккордеон (remember):"))
            .child(
                AnimatedVisibility::new(*expanded.get(), 0.2).child(
                    Text::new("Этот контент управляется remember.")
                        .padding(12.0)
                        .background(egui::Color32::from_gray(50)),
                ),
            )
            .child(Spacer::new(16.0))
            .child(Button::new("← Назад").on_click(RootMsg::Back).padding(8.0))
            .render(ui, dispatch);

        // Кнопки управления remember (вне Column)
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
