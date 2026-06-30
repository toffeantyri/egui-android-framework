//! ContainersScreen — демонстрация контейнеров.

use egui_android_framework::{
    containers::{Column, LazyColumn, Row, Stack},
    dispatcher::Dispatcher,
    modifiers::ModifierExt,
    widgets::{Button, Spacer, Text, Widget},
};

use crate::root_component::RootMsg;

/// Экран демонстрации контейнеров.
pub struct ContainersScreen;

impl ContainersScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, ui: &mut egui::Ui, _dispatch: &Dispatcher<RootMsg>) {
        Text::new("Контейнеры").render(ui, _dispatch);
        Spacer::new(8.0).render(ui, _dispatch);

        Text::new("Column (вертикально):").render(ui, _dispatch);
        Column::empty()
            .child(Text::new("A").background(egui::Color32::from_gray(50)))
            .child(Text::new("B").background(egui::Color32::from_gray(60)))
            .spacing(4.0)
            .render(ui, _dispatch);

        Spacer::new(8.0).render(ui, _dispatch);
        Text::new("Row (горизонтально):").render(ui, _dispatch);
        Row::empty()
            .child(Text::new("X").background(egui::Color32::from_gray(50)))
            .child(Text::new("Y").background(egui::Color32::from_gray(60)))
            .child(Text::new("Z").background(egui::Color32::from_gray(70)))
            .spacing(8.0)
            .render(ui, _dispatch);

        Spacer::new(8.0).render(ui, _dispatch);
        Text::new("Stack (наложение):").render(ui, _dispatch);
        Stack::new(vec![
            Box::new(Text::new("Фон").background(egui::Color32::BLUE)),
            Box::new(Text::new("Поверх").background(egui::Color32::RED)),
        ])
        .render(ui, _dispatch);

        Spacer::new(8.0).render(ui, _dispatch);
        Text::new("LazyColumn (список):").render(ui, _dispatch);
        let items: Vec<i32> = (1..=10).collect();
        LazyColumn::new(items, |item, _d| {
            Box::new(
                Text::new(format!("Элемент {}", item))
                    .padding(4.0)
                    .background(egui::Color32::from_gray(40)),
            )
        })
        .item_spacing(2.0)
        .render(ui, _dispatch);

        Spacer::new(16.0).render(ui, _dispatch);
        Button::new("Назад")
            .on_click(RootMsg::Back)
            .render(ui, _dispatch);
    }
}
