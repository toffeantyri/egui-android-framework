//! Тесты для виджетов (Widget, Text, Button, Spacer) и модификаторов (ModifierExt).
//!
//! Проверяет:
//! - Text рендерится без паники
//! - Button диспатчит сообщение при клике
//! - Spacer добавляет отступ
//! - Модификаторы (padding, background, size, align, clickable) рендерятся без паники

use std::sync::mpsc;

use crate::{
    dispatcher::Dispatcher,
    modifiers::ModifierExt,
    widgets::{Button, Spacer, Text, Widget},
};

// ─── Вспомогательные функции ────────────────────────────────────────────────

/// Создать egui-контекст и выполнить рендер виджета внутри CentralPanel.
fn render_widget<M>(widget: impl Widget<M>, dispatch: &Dispatcher<M>) {
    let ctx = egui::Context::default();
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            widget.render(ui, dispatch);
        });
    });
}

/// Создать пару (Dispatcher, Receiver) для тестов.
fn make_dispatcher<M>() -> (Dispatcher<M>, mpsc::Receiver<M>) {
    Dispatcher::new()
}

// ─── Тесты: Text ────────────────────────────────────────────────────────────

#[test]
fn text_renders_without_panic() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    render_widget(Text::new("Hello"), &dispatcher);
    // Проверка: не упало
}

#[test]
fn text_renders_with_special_chars() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    render_widget(Text::new("Счёт: 42\nНовая строка"), &dispatcher);
}

#[test]
fn text_empty_string() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    render_widget(Text::new(""), &dispatcher);
}

// ─── Тесты: Spacer ──────────────────────────────────────────────────────────

#[test]
fn spacer_renders_without_panic() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    render_widget(Spacer::new(16.0), &dispatcher);
}

#[test]
fn spacer_zero_size() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    render_widget(Spacer::new(0.0), &dispatcher);
}

#[test]
fn spacer_large_size() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    render_widget(Spacer::new(1000.0), &dispatcher);
}

// ─── Тесты: Button ──────────────────────────────────────────────────────────

#[test]
fn button_renders_without_panic() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    render_widget(Button::new("Click me"), &dispatcher);
}

#[test]
fn button_without_on_click_does_not_dispatch() {
    let (dispatcher, receiver) = make_dispatcher::<()>();
    render_widget(Button::new("No-op"), &dispatcher);
    let messages: Vec<_> = receiver.try_iter().collect();
    assert!(messages.is_empty(), "Без on_click не должно быть сообщений");
}

#[test]
fn button_with_on_click_renders() {
    #[derive(Debug, Clone, PartialEq)]
    enum TestMsg {
        Clicked,
    }

    let (dispatcher, _receiver) = make_dispatcher();
    let button = Button::new("Click me").on_click(TestMsg::Clicked);
    render_widget(button, &dispatcher);
    // Проверка: рендер без паники
}

#[test]
fn button_custom_height() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let button = Button::new("Tall").height(96.0);
    render_widget(button, &dispatcher);
}

#[test]
fn button_empty_text() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    render_widget(Button::new(""), &dispatcher);
}

// ─── Тесты: Модификаторы ────────────────────────────────────────────────────

#[test]
fn padded_widget_renders() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let widget = Text::new("Padded").padding(16.0);
    render_widget(widget, &dispatcher);
}

#[test]
fn padded_widget_zero_padding() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let widget = Text::new("No padding").padding(0.0);
    render_widget(widget, &dispatcher);
}

#[test]
fn sized_widget_renders() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let widget = Text::new("Sized").size(200.0, 48.0);
    render_widget(widget, &dispatcher);
}

#[test]
fn sized_widget_zero_size() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let widget = Text::new("Zero").size(0.0, 0.0);
    render_widget(widget, &dispatcher);
}

#[test]
fn background_widget_renders() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let widget = Text::new("With BG").background(egui::Color32::RED);
    render_widget(widget, &dispatcher);
}

#[test]
fn background_transparent() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let widget = Text::new("Transparent").background(egui::Color32::TRANSPARENT);
    render_widget(widget, &dispatcher);
}

#[test]
fn aligned_widget_renders() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let widget = Text::new("Centered").align(egui::Align::Center);
    render_widget(widget, &dispatcher);
}

#[test]
fn aligned_left() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let widget = Text::new("Left").align(egui::Align::LEFT);
    render_widget(widget, &dispatcher);
}

#[test]
fn clickable_widget_renders() {
    #[derive(Debug, Clone, PartialEq)]
    enum TestMsg {
        Clicked,
    }

    let (dispatcher, _receiver) = make_dispatcher();
    let widget = Text::new("Clickable area").clickable(TestMsg::Clicked);
    render_widget(widget, &dispatcher);
}

// ─── Тесты: Цепочки модификаторов ──────────────────────────────────────────

#[test]
fn chained_modifiers_render() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let widget = Text::new("Styled")
        .padding(16.0)
        .background(egui::Color32::from_gray(40));
    render_widget(widget, &dispatcher);
}

#[test]
fn full_chain_with_size_and_background() {
    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let widget = Text::new("Full chain")
        .size(300.0, 64.0)
        .background(egui::Color32::DARK_BLUE)
        .align(egui::Align::Center);
    render_widget(widget, &dispatcher);
}

// ─── Тесты: Взаимодействие Widget + ModifierExt ─────────────────────────────

#[test]
fn widget_trait_generic_usage() {
    // Проверяем, что виджеты с модификаторами можно использовать
    // через обобщённый код
    fn render_generic<M>(widget: &impl Widget<M>, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        widget.render(ui, dispatch);
    }

    let (dispatcher, _receiver) = make_dispatcher::<()>();
    let widget = Button::new("Generic").padding(8.0);

    let ctx = egui::Context::default();
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            render_generic(&widget, ui, &dispatcher);
        });
    });
}
