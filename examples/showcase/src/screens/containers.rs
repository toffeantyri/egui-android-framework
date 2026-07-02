//! ContainersScreen — демонстрация контейнеров Column, Row, Stack, LazyColumn.

use egui_android_framework::{
    runtime::Dispatcher,
    ui::{
        containers::{Column, LazyColumn, Row, Stack},
        modifier::ModifierExt,
        widgets::{Button, Spacer, Text, Widget},
    },
};

use crate::root_component::RootMsg;

/// Экран демонстрации контейнеров.
pub struct ContainersScreen;

impl ContainersScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<RootMsg>) {
        Column::<RootMsg>::empty()
            .child(Spacer::new(16.0))
            .child(Text::new("Контейнеры").padding(8.0))
            .child(Spacer::new(8.0))
            // Column (вертикально)
            .child(Text::new("Column (вертикально):"))
            .child(
                Column::<RootMsg>::empty()
                    .child(
                        Text::new("A")
                            .background(egui::Color32::from_gray(50))
                            .padding(4.0),
                    )
                    .child(Spacer::new(4.0))
                    .child(
                        Text::new("B")
                            .background(egui::Color32::from_gray(60))
                            .padding(4.0),
                    ),
            )
            .child(Spacer::new(8.0))
            // Row (горизонтально)
            .child(Text::new("Row (горизонтально):"))
            .child(
                Row::<RootMsg>::empty()
                    .child(
                        Text::new("X")
                            .background(egui::Color32::from_gray(50))
                            .padding(4.0),
                    )
                    .child(Spacer::new(8.0))
                    .child(
                        Text::new("Y")
                            .background(egui::Color32::from_gray(60))
                            .padding(4.0),
                    )
                    .child(Spacer::new(8.0))
                    .child(
                        Text::new("Z")
                            .background(egui::Color32::from_gray(70))
                            .padding(4.0),
                    ),
            )
            .child(Spacer::new(8.0))
            // Stack (наложение)
            .child(Text::new("Stack (наложение):"))
            .child(
                Stack::<RootMsg>::empty()
                    .child(
                        Text::new("Фон")
                            .background(egui::Color32::BLUE)
                            .padding(8.0),
                    )
                    .child(Text::new("Поверх").padding(8.0)),
            )
            .child(Spacer::new(8.0))
            // LazyColumn (список)
            .child(Text::new("LazyColumn (список):"))
            .child(LazyColumn::<RootMsg, i32>::new(
                (1..=10).collect(),
                |i, _dispatch| {
                    Box::new(
                        Text::new(format!("Элемент {}", i))
                            .background(egui::Color32::from_gray(40))
                            .padding(4.0),
                    )
                },
            ))
            .child(Spacer::new(16.0))
            .child(Button::new("← Назад").on_click(RootMsg::Back).padding(8.0))
            .render(ui, dispatch);
    }
}
