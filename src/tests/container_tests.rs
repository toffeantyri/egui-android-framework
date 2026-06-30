//! Тесты для контейнеров (Column, Row, Stack, LazyColumn).
//!
//! Проверяет:
//! - Column рендерит детей вертикально
//! - Row рендерит детей горизонтально
//! - Stack рендерит детей наложением
//! - LazyColumn рендерит все элементы в ScrollArea
//! - Контейнеры поддерживают модификаторы
//! - Цепочки builder-методов работают корректно

use crate::{
    containers::{Column, LazyColumn, Row, Stack},
    dispatcher::Dispatcher,
    modifiers::ModifierExt,
    widgets::{Text, Widget},
};

fn render_widget<M>(widget: impl Widget<M>, dispatch: &Dispatcher<M>) {
    let ctx = egui::Context::default();
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            widget.render(ui, dispatch);
        });
    });
}

fn make_dispatcher<M>() -> (Dispatcher<M>, std::sync::mpsc::Receiver<M>) {
    Dispatcher::new()
}

// ─── Column ─────────────────────────────────────────────────────────────────

#[test]
fn column_renders_children_vertically() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let column = Column::new(vec![
        Box::new(Text::new("First")),
        Box::new(Text::new("Second")),
    ]);
    render_widget(column, &dispatcher);
}

#[test]
fn column_empty_renders() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    render_widget(Column::<()>::empty(), &dispatcher);
}

#[test]
fn column_with_child_builder() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let column = Column::empty()
        .child(Text::new("A"))
        .child(Text::new("B"))
        .child(Text::new("C"));
    render_widget(column, &dispatcher);
}

#[test]
fn column_with_spacing() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let column =
        Column::new(vec![Box::new(Text::new("A")), Box::new(Text::new("B"))]).spacing(16.0);
    render_widget(column, &dispatcher);
}

// ─── Row ────────────────────────────────────────────────────────────────────

#[test]
fn row_renders_children_horizontally() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let row = Row::new(vec![Box::new(Text::new("A")), Box::new(Text::new("B"))]);
    render_widget(row, &dispatcher);
}

#[test]
fn row_empty_renders() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    render_widget(Row::<()>::empty(), &dispatcher);
}

#[test]
fn row_with_child_builder() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let row = Row::empty().child(Text::new("X")).child(Text::new("Y"));
    render_widget(row, &dispatcher);
}

#[test]
fn row_with_spacing() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let row = Row::new(vec![
        Box::new(Text::new("A")),
        Box::new(Text::new("B")),
        Box::new(Text::new("C")),
    ])
    .spacing(8.0);
    render_widget(row, &dispatcher);
}

// ─── Stack ──────────────────────────────────────────────────────────────────

#[test]
fn stack_renders_children_overlay() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let stack = Stack::new(vec![
        Box::new(Text::new("Bottom")),
        Box::new(Text::new("Top")),
    ]);
    render_widget(stack, &dispatcher);
}

#[test]
fn stack_empty_renders() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    render_widget(Stack::<()>::empty(), &dispatcher);
}

#[test]
fn stack_single_child() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let stack = Stack::new(vec![Box::new(Text::new("Lone"))]);
    render_widget(stack, &dispatcher);
}

// ─── LazyColumn ─────────────────────────────────────────────────────────────

#[test]
fn lazy_column_renders_all_items() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let items = vec![1, 2, 3, 4, 5];
    let lazy = LazyColumn::new(items, |item, _dispatch| {
        Box::new(Text::new(format!("Item {}", item)))
    });
    render_widget(lazy, &dispatcher);
}

#[test]
fn lazy_column_empty() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let items: Vec<i32> = vec![];
    let lazy = LazyColumn::new(items, |item, _dispatch| {
        Box::new(Text::new(format!("Item {}", item)))
    });
    render_widget(lazy, &dispatcher);
}

#[test]
fn lazy_column_with_custom_spacing() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let items = vec!["A", "B"];
    let lazy = LazyColumn::new(items, |item, _dispatch| {
        Box::new(Text::new(format!("Item {}", item)))
    })
    .item_spacing(16.0);
    render_widget(lazy, &dispatcher);
}

#[test]
fn lazy_column_single_item() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let items = vec![42];
    let lazy = LazyColumn::new(items, |item, _dispatch| {
        Box::new(Text::new(format!("Value: {}", item)))
    });
    render_widget(lazy, &dispatcher);
}

// ─── Контейнеры + модификаторы ─────────────────────────────────────────────

#[test]
fn column_with_padding_modifier() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let column = Column::new(vec![Box::new(Text::new("Hello"))]).padding(16.0);
    render_widget(column, &dispatcher);
}

#[test]
fn column_with_background_modifier() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let column = Column::new(vec![Box::new(Text::new("Hello"))]).background(egui::Color32::RED);
    render_widget(column, &dispatcher);
}

#[test]
fn row_with_modifiers_chain() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let row = Row::new(vec![Box::new(Text::new("Styled row"))])
        .padding(8.0)
        .background(egui::Color32::DARK_BLUE);
    render_widget(row, &dispatcher);
}

// ─── Вложенные контейнеры ──────────────────────────────────────────────────

#[test]
fn nested_column_in_column() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let inner = Column::new(vec![
        Box::new(Text::new("Inner A")),
        Box::new(Text::new("Inner B")),
    ]);
    let outer = Column::empty().child(Text::new("Outer")).child(inner);
    render_widget(outer, &dispatcher);
}

#[test]
fn row_in_column() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let column = Column::empty().child(Text::new("Header")).child(
        Row::new(vec![
            Box::new(Text::new("Left")),
            Box::new(Text::new("Right")),
        ])
        .spacing(16.0),
    );
    render_widget(column, &dispatcher);
}

#[test]
fn lazy_column_of_columns() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let items = vec!["A", "B"];
    let lazy = LazyColumn::new(items, |item, _dispatch| {
        Box::new(
            Column::new(vec![
                Box::new(Text::new(format!("Header {}", item))),
                Box::new(Text::new(format!("Body {}", item))),
            ])
            .padding(8.0),
        )
    });
    render_widget(lazy, &dispatcher);
}
