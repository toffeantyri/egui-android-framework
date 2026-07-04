//! ContainersScreen — демонстрация контейнеров Column, Row, Stack, LazyColumn.

use egui_android_framework::{
    runtime::Dispatcher,
    ui::{
        containers::{Column, LazyColumn, Row, Stack},
        modifier::ModifierExt,
        remember,
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
        Column::new()
            .scrollable()
            .show(ui, dispatch, |ui, dispatch| {
                Text::new("Контейнеры").padding(8.0).render(ui, dispatch);
                Spacer::new(8.0).render(ui, dispatch);

                // Column (вертикально)
                Text::new("Column (вертикально):").render(ui, dispatch);
                Column::new().show(ui, dispatch, |ui, dispatch| {
                    Text::new("A")
                        .background(egui::Color32::from_gray(50))
                        .padding(4.0)
                        .render(ui, dispatch);
                    Text::new("B")
                        .background(egui::Color32::from_gray(60))
                        .padding(4.0)
                        .render(ui, dispatch);
                });

                Spacer::new(8.0).render(ui, dispatch);

                // Row (горизонтально)
                Text::new("Row (горизонтально):").render(ui, dispatch);
                Row::new(ui, dispatch, |ui, dispatch| {
                    Text::new("X")
                        .background(egui::Color32::from_gray(50))
                        .padding(4.0)
                        .render(ui, dispatch);
                    Text::new("Y")
                        .background(egui::Color32::from_gray(60))
                        .padding(4.0)
                        .render(ui, dispatch);
                    Text::new("Z")
                        .background(egui::Color32::from_gray(70))
                        .padding(4.0)
                        .render(ui, dispatch);
                });

                Spacer::new(8.0).render(ui, dispatch);

                // Stack (наложение)
                Text::new("Stack (наложение):").render(ui, dispatch);
                Stack::new(ui, dispatch, |ui, dispatch| {
                    Text::new("Фон")
                        .background(egui::Color32::BLUE)
                        .padding(8.0)
                        .render(ui, dispatch);
                    Text::new("Поверх").padding(8.0).render(ui, dispatch);
                });

                Spacer::new(8.0).render(ui, dispatch);

                // LazyColumn с remember + on_click_with для каждого элемента
                Text::new("LazyColumn + on_click_with:").render(ui, dispatch);
                let items: Vec<i32> = (1..=5).collect();
                LazyColumn::new(items, ui, dispatch, |i, ui, dispatch| {
                    // remember для каждого элемента списка — уникальный ключ
                    let clicked = remember(ui, &format!("item_clicked_{}", i), || false);

                    Row::new(ui, dispatch, |ui, dispatch| {
                        Text::new(format!("Элемент {}", i))
                            .background(if *clicked.get() {
                                egui::Color32::from_rgb(0, 100, 200)
                            } else {
                                egui::Color32::from_gray(40)
                            })
                            .padding(6.0)
                            .render(ui, dispatch);

                        // Кнопка переключает remember напрямую через on_click_with
                        Button::new(if *clicked.get() { "✓" } else { "○" })
                            .on_click_with({
                                let clicked = clicked.clone();
                                move |_ui, _dispatch| {
                                    clicked.modify(|c| *c = !*c);
                                }
                            })
                            .padding(4.0)
                            .render(ui, dispatch);
                    });
                });

                Spacer::new(16.0).render(ui, dispatch);
                Button::new("← Назад")
                    .on_click(RootMsg::Back)
                    .padding(8.0)
                    .render(ui, dispatch);
            });
    }
}
