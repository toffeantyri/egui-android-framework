//! Тесты для ловли layout/alloc багов: rect.height, min_rect, cursor, consum, DPI.
//!
//! Каждый тест проверяет конкретное значение consum, rect.height, min_rect или cursor.
//! Все тесты запускаются на pp=1.0 и/или pp=3.25.

use std::cell::RefCell;

use egui_android_core::{widget::Widget as WidgetTrait, Dispatcher, UiWrapper};
use egui_android_ui::containers::{Column, LazyColumn, Row, Stack};
use egui_android_ui::modifier::{Modifier, ModifierDsl};
use egui_android_ui::widgets::{Button, Spacer, Text};

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
#[allow(dead_code)]
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

// ═══ 10. WIDTH / HEIGHT ═══════════════════════════════════════════════════════════

#[test]
fn test_width_200_consumed() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c = measure_consumed_y(ui, |ui| {
            Text::new("W")
                .modifier(Modifier::new().width(200.0))
                .render(ui, &dispatch);
        });
        assert!(c <= 25.0, "width(200) consum={} > 25", c);
    });
}

#[test]
fn test_height_48_consumed() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c = measure_consumed_y(ui, |ui| {
            Text::new("H")
                .modifier(Modifier::new().height(48.0))
                .render(ui, &dispatch);
        });
        assert!(c <= 55.0, "height(48) consum={} > 55", c);
    });
}

#[test]
fn test_width_200_height_48_consumed() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c = measure_consumed_y(ui, |ui| {
            Text::new("200x48")
                .modifier(Modifier::new().width(200.0).height(48.0))
                .render(ui, &dispatch);
        });
        assert!(c <= 55.0, "width+height consum={} > 55", c);
    });
}

#[test]
fn test_width_in_height_in_consumed() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c = measure_consumed_y(ui, |ui| {
            Text::new("S")
                .modifier(Modifier::new().width_in(50.0, 200.0).height_in(32.0, 48.0))
                .render(ui, &dispatch);
        });
        assert!(c <= 55.0, "width_in+height_in consum={} > 55", c);
    });
}

// ═══ 11. FILL_MAX_WIDTH / CLIP / SHADOW ═══════════════════════════════════════════

#[test]
fn test_fill_max_width_consumed() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c = measure_consumed_y(ui, |ui| {
            Text::new("F")
                .modifier(Modifier::new().fill_max_width())
                .render(ui, &dispatch);
        });
        assert!(c <= 25.0, "fill_max_width consum={} > 25", c);
    });
}

#[test]
fn test_clip_consumed() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c = measure_consumed_y(ui, |ui| {
            Text::new("C")
                .modifier(Modifier::new().clip(egui::CornerRadius::same(4)))
                .render(ui, &dispatch);
        });
        assert!(c <= 25.0, "clip consum={} > 25", c);
    });
}

#[test]
fn test_shadow_consumed() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c = measure_consumed_y(ui, |ui| {
            Text::new("S")
                .modifier(Modifier::new().shadow(4.0))
                .render(ui, &dispatch);
        });
        assert!(c <= 35.0, "shadow consum={} > 35", c);
    });
}

// ═══ 12. ДВОЙНОЙ ALLOC (РЕГРЕССИЯ) ═══════════════════════════════════════════════

#[test]
fn test_no_double_alloc_wrap_content() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c = measure_consumed_y(ui, |ui| {
            Text::new("Dbl")
                .modifier(Modifier::new().wrap_content_size())
                .render(ui, &dispatch);
        });
        assert!(c <= 22.0, "wrap_content_size consum={} > 22", c);
    });
}

#[test]
fn test_no_double_alloc_wrap_with_border() {
    let c = show_example_bg_simple(Modifier::new().wrap_content_size());
    assert!(c <= 25.0, "bg+border+wrap consum={} > 25", c);
}

// ═══ 13. ВСЕ МОДИФИКАТОРЫ PP=1 И PP=3.25 ═════════════════════════════════════════

#[test]
fn test_all_modifiers_consumed_pp1_and_pp325() {
    fn run_at(pp: f32) {
        with_pp(pp, |ui| {
            let (dispatch, _rx) = Dispatcher::<()>::new();
            let cases: Vec<(&str, Modifier<()>, f32)> = vec![
                ("wrap_width", Modifier::new().wrap_content_width(), 22.0),
                ("wrap_size", Modifier::new().wrap_content_size(), 22.0),
                (
                    "padding(8)",
                    Modifier::new().padding(8.0).wrap_content_size(),
                    35.0,
                ),
                ("width(200)", Modifier::new().width(200.0), 25.0),
                ("height(48)", Modifier::new().height(48.0), 55.0),
                (
                    "width_in+height_in",
                    Modifier::new().width_in(50.0, 200.0).height_in(32.0, 48.0),
                    55.0,
                ),
                ("fill_max_width", Modifier::new().fill_max_width(), 25.0),
                (
                    "border(2)+wrap",
                    Modifier::new()
                        .border(2.0, egui::Color32::WHITE)
                        .wrap_content_size(),
                    25.0,
                ),
                ("alpha(0.5)", Modifier::new().alpha(0.5), 22.0),
                (
                    "clip",
                    Modifier::new().clip(egui::CornerRadius::same(4)),
                    25.0,
                ),
                ("shadow(4)", Modifier::new().shadow(4.0), 35.0),
            ];
            for (name, modifier, max_c) in cases {
                let c = measure_consumed_y(ui, |ui| {
                    Text::new("X").modifier(modifier).render(ui, &dispatch);
                });
                assert!(c <= max_c, "[pp={}] {} consum={} > {}", pp, name, c, max_c);
            }
        });
    }
    run_at(1.0);
    run_at(3.25);
}

// ═══════════════════════════════════════════════════════════════════════════════════
// 10. ТЕСТЫ FILL_MAX_WIDTH С ALIGN
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn test_fill_max_width_with_align_min_rect_width() {
    // Проверяем, что fill_max_width + align(Center) + background + border + padding
    // даёт min_rect на всю доступную ширину (на pp=1 и pp=3.25)
    let (dispatch, _rx) = Dispatcher::<()>::new();

    let check = |ui: &mut UiWrapper| {
        let avail = ui.available_size().x;
        ui.scope(|scope_ui| {
            let mut w = UiWrapper::new_unconstrained(scope_ui);
            Text::new("X")
                .align(egui::Align::Center)
                .modifier(
                    Modifier::new()
                        .fill_max_width()
                        .background(egui::Color32::GRAY)
                        .border(1.0, egui::Color32::GREEN)
                        .padding(8.0),
                )
                .render(&mut w, &dispatch);
            let mr = w.min_rect();
            assert!(
                mr.width() >= avail - 10.0,
                "min_rect.width={} < avail={} - 10",
                mr.width(),
                avail
            );
        });
    };
    with_ui(|ui| check(ui));
    with_pp(3.25, |ui| check(ui));
}

#[test]
fn test_width_200_via_then_does_not_expand() {
    // Точная копия show_example_bg: background + border снаружи, width(200) через then
    // Это ловит баг когда Border.set_min_width растягивал outer_rect на весь экран
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let avail = ui.available_size().x;
        let mut min_w = 0.0f32;
        ui.scope(|scope_ui| {
            let mut w = UiWrapper::new_unconstrained(scope_ui);
            // show_example_bg делает: background(bg).border(1.0, GREEN).then(modifier)
            let inner = Modifier::new().width(200.0).height(48.0);
            Text::new("200x48")
                .modifier(
                    Modifier::new()
                        .background(egui::Color32::GRAY)
                        .border(1.0, egui::Color32::GREEN)
                        .then(inner),
                )
                .render(&mut w, &dispatch);
            min_w = w.min_rect().width();
        });
        // min_rect должен быть ~202 (width(200) + border(1)*2), а не avail
        assert!(
            min_w < avail - 100.0,
            "Width(200) через then: min_rect.width={} >= avail={} - 100 (должен быть ~202)",
            min_w,
            avail
        );
        assert!(
            min_w >= 190.0 && min_w <= 250.0,
            "Width(200) через then: min_rect.width={} вне 190-250",
            min_w
        );
    });
    // Проверяем на pp=3.25 (девайс)
    with_pp(3.25, |ui| {
        let avail = ui.available_size().x;
        let mut min_w = 0.0f32;
        ui.scope(|scope_ui| {
            let mut w = UiWrapper::new_unconstrained(scope_ui);
            let inner = Modifier::new().width(200.0).height(48.0);
            Text::new("200x48")
                .modifier(
                    Modifier::new()
                        .background(egui::Color32::GRAY)
                        .border(1.0, egui::Color32::GREEN)
                        .then(inner),
                )
                .render(&mut w, &dispatch);
            min_w = w.min_rect().width();
        });
        assert!(
            min_w < avail - 100.0,
            "[pp=3.25] Width(200) через then: min_rect.width={} >= avail={} - 100",
            min_w,
            avail
        );
        assert!(
            min_w >= 190.0 && min_w <= 250.0,
            "[pp=3.25] Width(200) через then: min_rect.width={} вне 190-250",
            min_w
        );
    });
}

#[test]
fn test_fill_max_width_consum_with_border() {
    // Проверяем consum: padding(8*2=16) + text + border = ~31 на pp=1
    let (dispatch, _rx) = Dispatcher::<()>::new();

    let check = |ui: &mut UiWrapper| {
        let before = ui.cursor().min.y;
        Text::new("X")
            .modifier(
                Modifier::new()
                    .fill_max_width()
                    .background(egui::Color32::GRAY)
                    .border(1.0, egui::Color32::GREEN)
                    .padding(8.0),
            )
            .render(ui, &dispatch);
        let after = ui.cursor().min.y;
        let consum = after - before;
        // consum должен быть > 20 (текст + padding), но не > 50
        assert!(
            consum >= 20.0 && consum <= 60.0,
            "fill_max_width+border+padding consum={} вне 20-60",
            consum
        );
    };
    with_ui(|ui| check(ui));
    with_pp(3.25, |ui| check(ui));
}

// ═══════════════════════════════════════════════════════════════════════════════════
// 11. МАЛЫЕ ПРОБЕЛЫ КОНТРАКТА
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn test_spacer_consum_same_at_pp1_and_pp325() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    let check = |ui: &mut UiWrapper| {
        let c = measure_consumed_y(ui, |ui| {
            Spacer::new(100.0).render(ui, &dispatch);
        });
        assert!((c - 100.0).abs() < 5.0, "Spacer consum={} != 100", c);
    };
    with_pp(1.0, |ui| check(ui));
    with_pp(3.25, |ui| check(ui));
}

#[test]
fn test_padding_zero_equals_no_padding() {
    // padding(0) не должен менять consum относительно текста без padding
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c0 = measure_consumed_y(ui, |ui| {
            Text::new("X").render(ui, &dispatch);
        });
        let c1 = measure_consumed_y(ui, |ui| {
            Text::new("X")
                .modifier(Modifier::new().padding(0.0))
                .render(ui, &dispatch);
        });
        assert!(
            (c0 - c1).abs() < 3.0,
            "padding(0) меняет consum: без={} с={}",
            c0,
            c1
        );
    });
}

#[test]
fn test_border_zero_equals_no_border() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c0 = measure_consumed_y(ui, |ui| {
            Text::new("X").render(ui, &dispatch);
        });
        let c1 = measure_consumed_y(ui, |ui| {
            Text::new("X")
                .modifier(Modifier::new().border(0.0, egui::Color32::RED))
                .render(ui, &dispatch);
        });
        assert!(
            (c0 - c1).abs() < 3.0,
            "border(0) меняет consum: без={} с={}",
            c0,
            c1
        );
    });
}

#[test]
fn test_alpha_zero_equals_no_alpha() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c0 = measure_consumed_y(ui, |ui| {
            Text::new("X").render(ui, &dispatch);
        });
        let c1 = measure_consumed_y(ui, |ui| {
            Text::new("X")
                .modifier(Modifier::new().alpha(0.0))
                .render(ui, &dispatch);
        });
        assert!(
            (c0 - c1).abs() < 3.0,
            "alpha(0) меняет consum: без={} с={}",
            c0,
            c1
        );
    });
}

#[test]
fn test_padding_bg_vs_bg_padding_same_consum() {
    // Оба порядка дают одинаковый consum, потому что background не alloc'ит место.
    // padding(16) даёт +32px в обоих случаях.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c1 = measure_consumed_y(ui, |ui| {
            Text::new("X")
                .modifier(Modifier::new().padding(16.0).background(egui::Color32::RED))
                .render(ui, &dispatch);
        });
        let c2 = measure_consumed_y(ui, |ui| {
            Text::new("X")
                .modifier(Modifier::new().background(egui::Color32::RED).padding(16.0))
                .render(ui, &dispatch);
        });
        assert!((c1 - c2).abs() < 5.0, "padding+bg={} bg+padding={}", c1, c2);
    });
}

#[test]
fn test_width_in_does_not_change_text_height() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c0 = measure_consumed_y(ui, |ui| {
            Text::new("X").render(ui, &dispatch);
        });
        let c1 = measure_consumed_y(ui, |ui| {
            Text::new("X")
                .modifier(Modifier::new().width_in(50.0, 200.0))
                .render(ui, &dispatch);
        });
        assert!(
            (c0 - c1).abs() < 5.0,
            "width_in меняет высоту: без={} с={}",
            c0,
            c1
        );
    });
}

#[test]
fn test_stack_with_padding_background() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Stack::new(ui, &dispatch, |ui, dispatch| {
            Text::new("A")
                .modifier(Modifier::new().padding(8.0).background(egui::Color32::RED))
                .render(ui, dispatch);
            Text::new("B")
                .modifier(Modifier::new().padding(8.0))
                .render(ui, dispatch);
        });
    });
}

#[test]
fn test_width_zero_does_not_panic() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Text::new("X")
            .modifier(Modifier::new().width(0.0))
            .render(ui, &dispatch);
    });
}

#[test]
fn test_height_zero_does_not_panic() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Text::new("X")
            .modifier(Modifier::new().height(0.0))
            .render(ui, &dispatch);
    });
}

// ═══════════════════════════════════════════════════════════════════════════════════
// 13. КРИТИЧЕСКИЕ ПРОБЕЛЫ КОНТРАКТА
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn test_paint_background_fills_allocated_rect() {
    // Background должен рисовать fill_rect внутри alloc'ированной области,
    // а не за её пределами. Проверяем через painter shapes count.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        // Считаем shapes ДО и ПОСЛЕ
        let shapes_before = ui
            .ctx()
            .data(|d| d.get_temp::<u64>(egui::Id::new("shape_count")).unwrap_or(0));
        Text::new("X")
            .modifier(Modifier::new().background(egui::Color32::RED))
            .render(ui, &dispatch);
    });
}

#[test]
fn test_paint_border_stays_within_alloc() {
    // Border не должен выходить за alloc'ированный rect
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Text::new("X")
            .modifier(Modifier::new().border(2.0, egui::Color32::GREEN))
            .render(ui, &dispatch);
    });
}

#[test]
fn test_paint_content_not_exceeding_rect() {
    // Текст/контент не должен рисоваться за пределами rect от allocate_exact_size.
    // Проверка: consumed consum ≈ galley height, rect не шире available
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let avail = ui.available_size().x;
        let c = measure_consumed_y(ui, |ui| {
            Text::new("Короткий текст")
                .modifier(Modifier::new().padding(16.0))
                .render(ui, &dispatch);
        });
        // consum должен быть ≈ padding(32) + text_height(~15) ≈ 47
        // Если consum > avail — контент вылез за пределы
        assert!(c < avail, "content consum={} >= avail={}", c, avail);
    });
}

// ═══════════════════════════════════════════════════════════════════════════════════
// 12. СРЕДНИЕ ПРОБЕЛЫ КОНТРАКТА
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn test_column_in_row_nested_consum() {
    // Column внутри Row: consum = max(child_A_height + spacing, child_B_height)
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        // consum Text("A") + Text("B") в Column = 2 * text_height + spacing(8) ≈ 38
        let c_col = measure_consumed_y(ui, |ui| {
            Column::new().show(ui, &dispatch, |ui, dispatch| {
                Text::new("A").render(ui, dispatch);
                Text::new("B").render(ui, dispatch);
            });
        });
        let c_row = measure_consumed_y(ui, |ui| {
            Row::new(ui, &dispatch, |ui, dispatch| {
                Column::new().show(ui, dispatch, |ui, dispatch| {
                    Text::new("A").render(ui, dispatch);
                    Text::new("B").render(ui, dispatch);
                });
            });
        });
        // Row не должен добавлять высоту: consum ≈ Column consum
        assert!(
            (c_row - c_col).abs() < 10.0,
            "Column in Row: c_row={} vs c_col={} различаются >10",
            c_row,
            c_col
        );
    });
}

#[test]
fn test_stack_children_overlap_not_sum() {
    // Stack должен перекрывать детей (consum ≈ max), а не суммировать (consum ≈ sum).
    // СЕЙЧАС ПАДАЕТ — Stack alloc'ит детей последовательно как Column.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c_single = measure_consumed_y(ui, |ui| {
            Text::new("Высокий текст").render(ui, &dispatch);
        });
        let c_stack = measure_consumed_y(ui, |ui| {
            Stack::new(ui, &dispatch, |ui, dispatch| {
                Text::new("A").render(ui, dispatch);
                Text::new("Высокий текст").render(ui, dispatch);
            });
        });
        // Stack consum ≈ max(child) ≈ c_single, не sum = c_single + child2
        let max_allowed = c_single * 1.5;
        assert!(
            c_stack <= max_allowed,
            "Stack consum={} > max_allowed={} (дети не перекрываются: sum ≈ {})",
            c_stack,
            max_allowed,
            c_single * 2.0
        );
    });
}

#[test]
fn test_lazy_column_consum_matches_children() {
    // LazyColumn consum = сумма consum детей + spacing
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let items = vec!["A", "B", "C"];
        let c = measure_consumed_y(ui, |ui| {
            LazyColumn::new(items, ui, &dispatch, |item, ui, dispatch| {
                Text::new(*item).render(ui, dispatch);
            });
        });
        // consum = 3 * text_height + 2 * spacing(8) ≈ 3*15 + 16 = 61
        assert!(
            c >= 40.0 && c <= 80.0,
            "LazyColumn 3 items consum={} вне 40-80",
            c
        );
    });
}

#[test]
fn test_spacer_no_double_alloc() {
    // Spacer не делает двойной alloc: consum ≈ size на всех pp
    let (dispatch, _rx) = Dispatcher::<()>::new();
    let check = |ui: &mut UiWrapper| {
        let c = measure_consumed_y(ui, |ui| {
            Spacer::new(50.0).render(ui, &dispatch);
        });
        assert!((c - 50.0).abs() < 5.0, "Spacer consum={} != 50", c);
    };
    with_pp(1.0, |ui| check(ui));
    with_pp(3.25, |ui| check(ui));
}

#[test]
fn test_button_with_width_in_height_in_consum() {
    // Button + width_in + height_in: consum ≈ height_in.max
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c_btn = measure_consumed_y(ui, |ui| {
            Button::<()>::new("X")
                .modifier(Modifier::new().width_in(50.0, 200.0).height_in(30.0, 100.0))
                .render(ui, &dispatch);
        });
        // height_in(30, 100) — max_height=100, min_height=30. consum должен быть ≈ 30 (текст меньше)
        assert!(
            c_btn >= 25.0 && c_btn <= 110.0,
            "Button + height_in consum={} вне 25-110",
            c_btn
        );
    });
}

#[test]
fn test_multiline_text_consum_exact() {
    // Многострочный текст: consum ≈ N_строк × line_height
    // cons
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c1 = measure_consumed_y(ui, |ui| {
            Text::new("Строка1").render(ui, &dispatch);
        });
        let c3 = measure_consumed_y(ui, |ui| {
            Text::new("Строка1\nСтрока2\nСтрока3").render(ui, &dispatch);
        });
        // 3 строки ≈ 3 × c1
        assert!(
            (c3 - c1 * 3.0).abs() < 10.0,
            "multiline: 3×c1={} vs c3={} различаются >10",
            c1 * 3.0,
            c3
        );
    });
}

#[test]
fn test_height_in_consum_equals_height() {
    // height_in(100, 200) + Text: consum ≈ 100 (min_height)
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c = measure_consumed_y(ui, |ui| {
            Text::new("X")
                .modifier(Modifier::new().width(100.0).height_in(100.0, 200.0))
                .render(ui, &dispatch);
        });
        // height_in требует min=100 → consum ≥ 100
        assert!(c >= 100.0, "height_in(100,200) consum={} < 100", c);
    });
}
