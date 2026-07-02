//! StateScreen — демонстрация локального состояния через remember.
//!
//! Показывает использование remember внутри замыканий Column.

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
            // remember ВНУТРИ замыкания Column
            // (теперь работает благодаря RwLock)
            // ──────────────────────────────────────
            Text::new("remember внутри замыкания:").render(ui, dispatch);

            let count = remember(ui, "ss_count", || 0i32);
            Text::new(format!("Счётчик: {}", count.get()))
                .padding(12.0)
                .background(egui::Color32::from_gray(60))
                .render(ui, dispatch);

            let expanded = remember(ui, "ss_expanded", || false);

            Text::new("Состояние expanded:").render(ui, dispatch);
            Text::new(format!(
                "{}",
                if *expanded.get() { "true" } else { "false" }
            ))
            .padding(8.0)
            .background(egui::Color32::from_gray(50))
            .render(ui, dispatch);

            Spacer::new(4.0).render(ui, dispatch);

            Text::new("Значения remember сохраняются между кадрами").render(ui, dispatch);
            Text::new("и сбрасываются при пересоздании контекста egui.").render(ui, dispatch);

            Spacer::new(16.0).render(ui, dispatch);
            Button::new("← Назад")
                .on_click(RootMsg::Back)
                .padding(8.0)
                .render(ui, dispatch);
        });
    }
}
