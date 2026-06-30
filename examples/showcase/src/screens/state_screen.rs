//! StateScreen — демонстрация локального состояния через remember.

use egui_android_framework::{
    dispatcher::Dispatcher,
    modifiers::ModifierExt,
    remember,
    widgets::{Button, Spacer, Text, Widget},
};

use crate::root_component::RootMsg;

/// Экран демонстрации локального состояния.
pub struct StateScreen;

impl StateScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, ui: &mut egui::Ui, _dispatch: &Dispatcher<RootMsg>) {
        let mut expanded = remember(ui, "ss_expanded", || false);
        let mut local_count = remember(ui, "ss_local_count", || 0i32);

        Text::new("Локальное состояние (remember)").render(ui, _dispatch);
        Spacer::new(8.0).render(ui, _dispatch);

        Text::new("Счётчик (локальный):").render(ui, _dispatch);
        Text::new(format!("{}", *local_count.get()))
            .padding(16.0)
            .background(egui::Color32::from_gray(60))
            .render(ui, _dispatch);

        if ui.button("+").clicked() {
            local_count.modify(|c| *c += 1);
        }
        if ui.button("-").clicked() {
            local_count.modify(|c| *c -= 1);
        }

        Spacer::new(8.0).render(ui, _dispatch);
        Text::new("Аккордеон (remember):").render(ui, _dispatch);
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

        if *expanded.get() {
            Text::new("Этот контент управляется remember.")
                .padding(12.0)
                .background(egui::Color32::from_gray(50))
                .render(ui, _dispatch);
        }

        Spacer::new(16.0).render(ui, _dispatch);
        Button::new("Назад")
            .on_click(RootMsg::Back)
            .render(ui, _dispatch);
    }
}
