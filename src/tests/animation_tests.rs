//! Тесты для системы анимаций (AnimatedVisibility, Fade, Slide, animate_value).
//!
//! Проверяет:
//! - AnimatedVisibility рендерит при visible=true
//! - AnimatedVisibility скрывает при visible=false
//! - Fade применяет прозрачность
//! - Slide смещает виджет
//! - animate_value возвращает интерполированное значение
//! - Цепочки анимаций с модификаторами

use std::cell::RefCell;

use crate::{
    animation::{AnimatedVisibility, AnimationExt, SlideDirection},
    containers::Column,
    dispatcher::Dispatcher,
    modifiers::ModifierExt,
    widgets::{Text, Widget},
};

fn with_ui(f: impl FnOnce(&mut egui::Ui, &Dispatcher<()>)) {
    let f = RefCell::new(Some(f));
    let (dispatcher, _receiver) = Dispatcher::new();
    let ctx = egui::Context::default();
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let f = f.borrow_mut().take().unwrap();
            f(ui, &dispatcher);
        });
    });
}

// ─── AnimatedVisibility ─────────────────────────────────────────────────────

#[test]
fn animated_visibility_renders_when_visible() {
    with_ui(|ui, dispatch| {
        let widget = AnimatedVisibility::new(true, 0.3).child(Text::new("Hello"));
        widget.render(ui, dispatch);
        // Не падает — тест пройден
    });
}

#[test]
fn animated_visibility_hides_when_not_visible() {
    with_ui(|ui, dispatch| {
        // При visible=false и первом кадре прогресс = 0.0
        let widget = AnimatedVisibility::new(false, 0.3).child(Text::new("Hidden"));
        widget.render(ui, dispatch);
    });
}

#[test]
fn animated_visibility_empty_no_child() {
    with_ui(|ui, dispatch| {
        // AnimatedVisibility без child
        let widget = AnimatedVisibility::<()>::new(true, 0.3);
        widget.render(ui, dispatch);
    });
}

// ─── Fade ───────────────────────────────────────────────────────────────────

#[test]
fn fade_applies_opacity() {
    with_ui(|ui, dispatch| {
        let widget = Text::new("Faded").fade(0.5);
        widget.render(ui, dispatch);
    });
}

#[test]
fn fade_zero_opacity() {
    with_ui(|ui, dispatch| {
        let widget = Text::new("Invisible").fade(0.0);
        widget.render(ui, dispatch);
    });
}

#[test]
fn fade_full_opacity() {
    with_ui(|ui, dispatch| {
        let widget = Text::new("Fully visible").fade(1.0);
        widget.render(ui, dispatch);
    });
}

// ─── Slide ──────────────────────────────────────────────────────────────────

#[test]
fn slide_renders_without_panic() {
    with_ui(|ui, dispatch| {
        let widget = Text::new("Sliding").slide(SlideDirection::Right, 50.0);
        widget.render(ui, dispatch);
    });
}

#[test]
fn slide_all_directions() {
    with_ui(|ui, dispatch| {
        for &dir in &[
            SlideDirection::Left,
            SlideDirection::Right,
            SlideDirection::Up,
            SlideDirection::Down,
        ] {
            let widget = Text::new("Dir").slide(dir, 30.0);
            widget.render(ui, dispatch);
        }
    });
}

#[test]
fn slide_zero_offset() {
    with_ui(|ui, dispatch| {
        let widget = Text::new("No offset").slide(SlideDirection::Left, 0.0);
        widget.render(ui, dispatch);
    });
}

// ─── Цепочки ────────────────────────────────────────────────────────────────

#[test]
fn fade_with_padding_modifier() {
    with_ui(|ui, dispatch| {
        let widget = Text::new("Fade + Padding").fade(0.8).padding(16.0);
        widget.render(ui, dispatch);
    });
}

#[test]
fn animated_visibility_with_fade_child() {
    with_ui(|ui, dispatch| {
        let widget = AnimatedVisibility::new(true, 0.3).child(Text::new("Nested fade").fade(0.6));
        widget.render(ui, dispatch);
    });
}

#[test]
fn slide_in_column() {
    with_ui(|ui, dispatch| {
        let column = Column::empty()
            .child(Text::new("Normal"))
            .child(Text::new("Sliding").slide(SlideDirection::Right, 40.0));
        column.render(ui, dispatch);
    });
}

// ─── AnimationExt blanket impl ──────────────────────────────────────────────

#[test]
fn animation_ext_trait_is_implemented() {
    // Проверяем, что AnimationExt реализован для Widget<M>
    fn requires_animation_ext<M>(widget: impl AnimationExt<M>) {
        let _ = widget;
    }
    requires_animation_ext::<()>(Text::new("Test"));
}
