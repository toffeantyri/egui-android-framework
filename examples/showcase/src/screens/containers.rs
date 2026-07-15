//! ContainersScreen — демонстрация контейнеров Column, Row, Stack, LazyColumn.

use egui_android_framework::{
    core::{Component as UiComponent, LifecycleObserver},
    runtime::Dispatcher,
    ui::{
        containers::{Align, Column, LazyColumn, Row, Stack},
        modifier::{Modifier, ModifierDsl},
        remember,
        theme::Theme,
        widgets::{Button, Spacer, Text, Widget},
        UiWrapper,
    },
};

use crate::navigation_host::RootMsg;

/// Экран демонстрации контейнеров.
pub struct ContainersScreen;

impl ContainersScreen {
    pub fn new() -> Self {
        Self
    }
}

impl LifecycleObserver for ContainersScreen {}

impl UiComponent for ContainersScreen {
    type State = ();
    type Message = RootMsg;

    fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<Self::Message>) {
        let c = &Theme::current_from_ui(ui).colors;

        Column::new()
            .scrollable()
            .show(ui, dispatch, |ui, dispatch| {
                Text::new("Контейнеры")
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);
                Spacer::new(8.0).render(ui, dispatch);

                // Column (вертикально)
                Text::new("Column (вертикально):").render(ui, dispatch);
                Column::new().show(ui, dispatch, |ui, dispatch| {
                    Text::new("A")
                        .text_color(c.on_secondary)
                        .modifier(
                            Modifier::new()
                                .fill_max_width()
                                .background(c.secondary)
                                .padding(4.0),
                        )
                        .render(ui, dispatch);
                    Text::new("B")
                        .text_color(c.on_secondary)
                        .modifier(
                            Modifier::new()
                                .fill_max_width()
                                .background(c.secondary)
                                .padding(4.0),
                        )
                        .render(ui, dispatch);
                    Text::new("C")
                        .text_color(c.on_secondary)
                        .modifier(
                            Modifier::new()
                                .fill_max_width()
                                .background(c.secondary)
                                .padding(4.0),
                        )
                        .render(ui, dispatch);
                });

                Spacer::new(8.0).render(ui, dispatch);

                // Row (горизонтально)
                Text::new("Row (горизонтально):").render(ui, dispatch);
                Row::new(ui, dispatch, |ui, dispatch| {
                    Text::new("X")
                        .text_color(c.on_secondary)
                        .modifier(Modifier::new().background(c.secondary).padding(4.0))
                        .render(ui, dispatch);
                    Text::new("Y")
                        .text_color(c.on_secondary)
                        .modifier(Modifier::new().background(c.secondary).padding(4.0))
                        .render(ui, dispatch);
                    Text::new("Z")
                        .text_color(c.on_secondary)
                        .modifier(Modifier::new().background(c.secondary).padding(4.0))
                        .render(ui, dispatch);
                });

                Spacer::new(8.0).render(ui, dispatch);

                // Stack (наложение)
                Text::new("Stack (наложение):").render(ui, dispatch);
                Stack::new()
                    .add(
                        Text::new("Фон")
                            .text_color(c.on_primary)
                            .modifier(Modifier::new().background(c.primary).padding(8.0)),
                    )
                    .add_with_align(
                        Text::new("Поверх").modifier(Modifier::new().padding(8.0)),
                        Align::Center,
                    )
                    .show(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // LazyColumn с remember + on_click_with для каждого элемента
                Text::new("LazyColumn + on_click_with:").render(ui, dispatch);
                let items: Vec<i32> = (1..=10).collect();
                LazyColumn::new(items, ui, dispatch, |i, ui, dispatch| {
                    let clicked = remember(ui, format!("item_clicked_{}", i), || false);

                    Row::new(ui, dispatch, |ui, dispatch| {
                        let bg = if *clicked.get() {
                            c.primary
                        } else {
                            c.secondary
                        };
                        let fg = if *clicked.get() {
                            c.on_primary
                        } else {
                            c.on_secondary
                        };

                        Text::new(format!("Элемент {}", i))
                            .text_color(fg)
                            .modifier(Modifier::new().background(bg).padding(6.0))
                            .render(ui, dispatch);

                        Button::new(if *clicked.get() { "✓" } else { "○" })
                            .on_click_with({
                                let clicked = clicked.clone();
                                move |_ui, _dispatch| {
                                    clicked.modify(|c| *c = !*c);
                                }
                            })
                            .theme_colors(c.secondary)
                            .text_color(c.on_secondary)
                            .modifier(Modifier::new().padding(4.0))
                            .render(ui, dispatch);
                    });
                });

                Spacer::new(16.0).render(ui, dispatch);
                Button::new("← Назад")
                    .on_click(RootMsg::Back)
                    .theme_colors(c.primary)
                    .text_color(c.on_primary)
                    .modifier(Modifier::new().fill_max_width().padding(8.0))
                    .render(ui, dispatch);
            });
    }

    fn handle(&mut self, _msg: Self::Message) {}

    fn state(&self) -> &Self::State {
        &()
    }
}
