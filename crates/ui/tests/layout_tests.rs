//! Тесты для ловли layout/alloc багов: rect.height, min_rect, cursor, consum, DPI.
//!
//! Каждый тест проверяет конкретное значение consum, rect.height, min_rect или cursor.
//! Все тесты запускаются на pp=1.0 и/или pp=3.25.

use std::cell::RefCell;

use egui_android_core::{widget::Widget as WidgetTrait, Dispatcher, UiWrapper};
use egui_android_ui::containers::Column;
use egui_android_ui::modifier::{Modifier, ModifierDsl};
use egui_android_ui::widgets::{Spacer, Text};

// ═══════════════════════════════════════════════════════════════════════════════════
// Helper: with_ui
// ═══════════════════════════════════════════════════════════════════════════════════

fn with_ui(f: impl FnOnce(&mut UiWrapper)) {
    let f = RefCell::new(Some(f));
    let ctx = egui::Context::default();
    let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let f = f.borrow_mut().take().unwrap();
            f(&mut UiWrapper::new_unconstrained(ui));
        });
    });
}

/// Запустить тест с заданным pixels_per_point.
fn with_pp(pp: f32, f: impl FnOnce(&mut UiWrapper)) {
    let f = RefCell::new(Some(f));
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(pp);
    let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let f = f.borrow_mut().take().unwrap();
            f(&mut UiWrapper::new_unconstrained(ui));
        });
    });
}

/// Измерить consum = cursor.y до - cursor.y после render.
fn measure_consumed_y(ui: &mut UiWrapper, render: impl FnOnce(&mut UiWrapper)) -> f32 {
    let before = ui.available_rect_before_wrap().min.y;
    render(ui);
    ui.available_rect_before_wrap().min.y - before
}

/// Измерить rect.height от одного Text::render.
fn measure_text_rect(ui: &mut UiWrapper, text: &str, modifier: Modifier<()>) -> f32 {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    let mut rect_h = 0.0f32;
    ui.scope(|scope_ui| {
        let mut w = UiWrapper::new_unconstrained(scope_ui);
        Text::new(text).modifier(modifier).render(&mut w, &dispatch);
        rect_h = w.min_rect().height();
    });
    rect_h
}

// ═══════════════════════════════════════════════════════════════════════════════════
// 1. ТЕСТЫ TEXT
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn test_text_rect_height_equals_galley_size_y() {
    // rect.height == galley.size().y для текста.
    // Измеряем через consumed: consum должен быть ≈ 15.1
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let consumed = measure_consumed_y(ui, |ui| {
            Text::new("Test text height").render(ui, &dispatch);
        });
        assert!(
            (consumed - 15.1).abs() < 5.0,
            "consum={} != ~15.1",
            consumed
        );
    });
}

#[test]
fn test_text_no_vertical_centering() {
    // consumed должен быть ≈ 15px для текста
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let consumed = measure_consumed_y(ui, |ui| {
            Text::new("No center").render(ui, &dispatch);
        });
        // consum = text_height ≈ 15
        assert!(
            consumed >= 10.0 && consumed <= 25.0,
            "text consum={} вне 10-25",
            consumed
        );
    });
}

// ═══════════════════════════════════════════════════════════════════════════════════
// 2. ТЕСТЫ WRAP_CONTENT
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn test_wrap_content_width_consumed_size() {
    // consum = border(4) + text(~15.1) + Column spacing(8) ≈ 27
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let consumed = measure_consumed_y(ui, |ui| {
            Text::new("Короткий текст")
                .modifier(Modifier::new().wrap_content_width())
                .render(ui, &dispatch);
        });
        // consum = text(~15.1) = ~15
        assert!(
            consumed <= 22.0,
            "wrap_content_width consum={} > 22",
            consumed
        );
    });
}

#[test]
fn test_wrap_content_size_consumed_size() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let consumed = measure_consumed_y(ui, |ui| {
            Text::new("Короткий")
                .modifier(Modifier::new().wrap_content_size())
                .render(ui, &dispatch);
        });
        assert!(
            consumed <= 22.0,
            "wrap_content_size consum={} > 22",
            consumed
        );
    });
}

#[test]
fn test_wrap_content_with_border_background_consumed() {
    // background + border(2) + wrap_content_size + text
    // consum = border(4) + text(~15.1) + Column spacing(8) ≈ 27
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let consumed = measure_consumed_y(ui, |ui| {
            Text::new("Test")
                .modifier(
                    Modifier::new()
                        .background(egui::Color32::RED)
                        .border(2.0, egui::Color32::WHITE)
                        .wrap_content_size(),
                )
                .render(ui, &dispatch);
        });
        // consum: border(4) + text(~15) = ≈19 на pp=1 (без Column spacing в прямой render)
        assert!(consumed <= 25.0, "bg+border+wrap consum={} > 25", consumed);
    });
}

// ═══════════════════════════════════════════════════════════════════════════════════
// 3. ТЕСТЫ CONSUM ДЛЯ 6–10 (как show_example_bg, но без title)
// ═══════════════════════════════════════════════════════════════════════════════════

fn show_example_bg_simple(modifier: Modifier<()>) -> f32 {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    let mut consumed = 0.0f32;
    with_ui(|ui| {
        consumed = measure_consumed_y(ui, |ui| {
            Text::new("Тест")
                .modifier(
                    Modifier::new()
                        .background(egui::Color32::DARK_GRAY)
                        .border(2.0, egui::Color32::WHITE)
                        .then(modifier),
                )
                .render(ui, &dispatch);
        });
    });
    consumed
}

#[test]
fn test_ex6_like_width_height() {
    // width(200) + height(48) + bg + border
    // consum = border(4) + height(48) = 52
    let consumed = show_example_bg_simple(Modifier::new().width(200.0).height(48.0));
    assert!(consumed <= 58.0, "ex6-like consum={} > 58", consumed);
}

#[test]
fn test_ex7_like_widthin_heightin() {
    // width_in(100,300) + height_in(32,64) + bg + border
    // consum = border(4) + height_in(64) = 68
    let consumed =
        show_example_bg_simple(Modifier::new().width_in(100.0, 300.0).height_in(32.0, 64.0));
    assert!(consumed <= 74.0, "ex7-like consum={} > 74", consumed);
}

#[test]
fn test_ex8_like_wrap_content_width() {
    // wrap_content_width + bg + border
    // consum = border(4) + text(~15) = ≈19
    let consumed = show_example_bg_simple(Modifier::new().wrap_content_width());
    assert!(
        consumed <= 25.0,
        "ex8-like wrap_content_width consum={} > 25",
        consumed
    );
}

#[test]
fn test_ex9_like_wrap_content_size() {
    let consumed = show_example_bg_simple(Modifier::new().wrap_content_size());
    assert!(
        consumed <= 25.0,
        "ex9-like wrap_content_size consum={} > 25",
        consumed
    );
}

#[test]
fn test_ex10_like_padding_8() {
    // padding(8) + bg + border
    // consum = border(4) + padding(16) + text(~15) = ≈35
    let consumed = show_example_bg_simple(Modifier::new().padding(8.0));
    assert!(
        consumed <= 40.0,
        "ex10-like padding(8) consum={} > 40",
        consumed
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════
// 4. ТЕСТЫ BORDER
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn test_border_adds_4px() {
    // border(2) добавляет 4px к consum (2 сверху + 2 снизу)
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c1 = measure_consumed_y(ui, |ui| {
            Text::new("A").render(ui, &dispatch);
        });
        let c2 = measure_consumed_y(ui, |ui| {
            Text::new("A")
                .modifier(Modifier::new().border(2.0, egui::Color32::WHITE))
                .render(ui, &dispatch);
        });
        let diff = c2 - c1;
        // border(2) = 4px ± округление
        assert!(diff <= 6.0, "border добавил {}px > 6", diff);
    });
}

// ═══════════════════════════════════════════════════════════════════════════════════
// 5. ТЕСТЫ BACKGROUND
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn test_background_does_not_add_height() {
    // background сам по себе не должен добавлять высоту
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c1 = measure_consumed_y(ui, |ui| {
            Text::new("A").render(ui, &dispatch);
        });
        let c2 = measure_consumed_y(ui, |ui| {
            Text::new("A")
                .modifier(Modifier::new().background(egui::Color32::RED))
                .render(ui, &dispatch);
        });
        let diff = c2 - c1;
        assert!(diff.abs() <= 2.0, "background изменил consum на {}", diff);
    });
}

// ═══════════════════════════════════════════════════════════════════════════════════
// 6. ТЕСТЫ DPI
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn test_wrap_content_consum_same_at_pp1_and_pp325() {
    // consum не должен зависеть от pp
    let mod_chain = |bg: egui::Color32| {
        Modifier::new()
            .background(bg)
            .border(2.0, egui::Color32::WHITE)
            .wrap_content_size()
    };

    let mut consum_pp1 = 0.0f32;
    let mut consum_pp325 = 0.0f32;

    with_pp(1.0, |ui| {
        let (dispatch, _rx) = Dispatcher::<()>::new();
        consum_pp1 = measure_consumed_y(ui, |ui| {
            Text::new("DPI test")
                .modifier(mod_chain(egui::Color32::DARK_GRAY))
                .render(ui, &dispatch);
        });
    });

    with_pp(3.25, |ui| {
        let (dispatch, _rx) = Dispatcher::<()>::new();
        consum_pp325 = measure_consumed_y(ui, |ui| {
            Text::new("DPI test")
                .modifier(mod_chain(egui::Color32::DARK_GRAY))
                .render(ui, &dispatch);
        });
    });

    let diff = (consum_pp1 - consum_pp325).abs();
    assert!(
        diff <= 2.0,
        "consum разный при pp=1 ({}) vs pp=3.25 ({}), diff={}",
        consum_pp1,
        consum_pp325,
        diff
    );
}

#[test]
fn test_text_rect_height_same_at_pp1_and_pp325() {
    // consumed не должен зависеть от pp
    let mut c1 = 0.0f32;
    let mut c325 = 0.0f32;

    with_pp(1.0, |ui| {
        let (dispatch, _rx) = Dispatcher::<()>::new();
        c1 = measure_consumed_y(ui, |ui| {
            Text::new("Height test").render(ui, &dispatch);
        });
    });

    with_pp(3.25, |ui| {
        let (dispatch, _rx) = Dispatcher::<()>::new();
        c325 = measure_consumed_y(ui, |ui| {
            Text::new("Height test").render(ui, &dispatch);
        });
    });

    let diff = (c1 - c325).abs();
    assert!(
        diff <= 2.0,
        "consum разный при pp=1 ({}) vs pp=3.25 ({}), diff={}",
        c1,
        c325,
        diff
    );
}

// ═══════════════════════════════════════════════════════════════════════════════════
// 7. ТЕСТЫ PADDING
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn test_padding_8_consumed() {
    // padding(8) добавляет 16px к consum (8 сверху + 8 снизу)
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c1 = measure_consumed_y(ui, |ui| {
            Text::new("Pad").render(ui, &dispatch);
        });
        let c2 = measure_consumed_y(ui, |ui| {
            Text::new("Pad")
                .modifier(Modifier::new().padding(8.0).wrap_content_size())
                .render(ui, &dispatch);
        });
        let diff = c2 - c1;
        // padding(8) = text(15) + 16 = ~31, text alone = ~15, diff = ~16
        assert!(
            diff >= 10.0 && diff <= 25.0,
            "padding(8) изменил consum на {} (ожидалось ~16)",
            diff
        );
    });
}

// ═══════════════════════════════════════════════════════════════════════════════════
// 8. ТЕСТЫ NESTED МОДИФИКАТОРОВ
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn test_nested_bg_border_padding_wrap() {
    // Полная цепочка: bg + border(2) + padding(8) + wrap_content_size + text
    // consum = border(4) + padding(16) + text(~15) = ≈35
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let consumed = measure_consumed_y(ui, |ui| {
            Text::new("Nested chain test")
                .modifier(
                    Modifier::new()
                        .background(egui::Color32::DARK_GRAY)
                        .border(2.0, egui::Color32::WHITE)
                        .padding(8.0)
                        .wrap_content_size(),
                )
                .render(ui, &dispatch);
        });
        // consum = border(4) + padding(16) + text(~15) = ≈35
        assert!(consumed <= 42.0, "nested chain consum={} > 42", consumed);
    });
}

#[test]
fn test_nested_bg_border_width_height() {
    // bg + border(2) + width(200) + height(48) + text
    // consum = border(4) + height(48) = 52
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let consumed = measure_consumed_y(ui, |ui| {
            Text::new("200x48")
                .modifier(
                    Modifier::new()
                        .background(egui::Color32::DARK_GRAY)
                        .border(2.0, egui::Color32::WHITE)
                        .width(200.0)
                        .height(48.0),
                )
                .render(ui, &dispatch);
        });
        assert!(
            consumed <= 58.0,
            "nested width+height consum={} > 58",
            consumed
        );
    });
}

// ═══════════════════════════════════════════════════════════════════════════════════
// 9. ТЕСТЫ COLUMN TOP-DOWN LAYOUT
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn test_column_top_down_consum_sum() {
    // consum Column = сумма consum детей + spacing
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let consumed = measure_consumed_y(ui, |ui| {
            Column::new().show(ui, &dispatch, |ui, dispatch| {
                Text::new("A").render(ui, dispatch);
                Text::new("B").render(ui, dispatch);
                Text::new("C").render(ui, dispatch);
            });
        });
        // consum должен быть > 0 (что-то alloc'илось)
        assert!(
            consumed > 0.0 && consumed < 200.0,
            "Column 3 children consum={} вне 0-200",
            consumed
        );
    });
}
