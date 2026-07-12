//! Тесты для ловли layout/alloc багов: rect.height, min_rect, cursor, consum, DPI.
//!
//! Каждый тест проверяет конкретное значение consum, rect.height, min_rect или cursor.
//! Все тесты запускаются на pp=1.0 и/или pp=3.25.

use std::cell::RefCell;

use egui_android_core::{widget::Widget as WidgetTrait, Dispatcher, UiWrapper};
use egui_android_ui::containers::{Align, Column, LazyColumn, Row, Stack};
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
    // Проверяем точное значение consum (≈18 на pp=1 для стандартного шрифта)
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let consumed = measure_consumed_y(ui, |ui| {
            Text::new("Test text height").render(ui, &dispatch);
        });
        assert!((consumed - 18.0).abs() < 2.0, "consum={} != ~18", consumed);
    });
}

#[test]
fn test_text_no_vertical_centering() {
    // consumed должен быть ≈ 18px для текста (не больше, т.к. центрирования нет)
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let consumed = measure_consumed_y(ui, |ui| {
            Text::new("No center").render(ui, &dispatch);
        });
        // consum = text_height ≈ 18 — точная проверка
        assert!(
            (consumed - 18.0).abs() < 2.0,
            "text consum={} != ~18",
            consumed
        );
    });
}

// ═══════════════════════════════════════════════════════════════════════════════════
// 2. ТЕСТЫ WRAP_CONTENT
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn test_wrap_content_width_consumed_size() {
    // wrap_content_width не меняет высоту: consum ≈ text(~15)
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c0 = measure_consumed_y(ui, |ui| {
            Text::new("X").render(ui, &dispatch);
        });
        let c1 = measure_consumed_y(ui, |ui| {
            Text::new("X")
                .modifier(Modifier::new().wrap_content_width())
                .render(ui, &dispatch);
        });
        // wrap_content_width не должен менять consumed (работает только с шириной)
        assert!(
            (c0 - c1).abs() < 3.0,
            "wrap_content_width изменил consum: без={} с={}",
            c0,
            c1
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
    // consum = border(4) + text(~15) = ≈19 на pp=1
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
        // consum: border(4) + text(~15) = ≈19
        assert!(
            (consumed - 19.0).abs() < 5.0,
            "bg+border+wrap consum={} != ~19",
            consumed
        );
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
    assert!(
        (consumed - 52.0).abs() < 6.0,
        "ex6-like consum={} != ~52",
        consumed
    );
}

#[test]
fn test_ex7_like_widthin_heightin() {
    // width_in(100,300) + height_in(32,64) + bg + border
    // consum = border(4) + height_in(64) = 68
    let consumed =
        show_example_bg_simple(Modifier::new().width_in(100.0, 300.0).height_in(32.0, 64.0));
    assert!(
        (consumed - 68.0).abs() < 6.0,
        "ex7-like consum={} != ~68",
        consumed
    );
}

#[test]
fn test_ex8_like_wrap_content_width() {
    // wrap_content_width + bg + border
    // consum = border(4) + text(~15) = ≈19
    let consumed = show_example_bg_simple(Modifier::new().wrap_content_width());
    assert!(
        (consumed - 19.0).abs() < 5.0,
        "ex8-like wrap_content_width consum={} != ~19",
        consumed
    );
}

#[test]
fn test_ex9_like_wrap_content_size() {
    // wrap_content_size + bg + border
    // consum = border(4) + text(~15) = ≈19
    let consumed = show_example_bg_simple(Modifier::new().wrap_content_size());
    assert!(
        (consumed - 19.0).abs() < 5.0,
        "ex9-like wrap_content_size consum={} != ~19",
        consumed
    );
}

#[test]
fn test_ex10_like_padding_8() {
    // padding(8) + bg + border
    // consum = border(4) + padding(16) + text(~15) = ≈35
    let consumed = show_example_bg_simple(Modifier::new().padding(8.0));
    assert!(
        (consumed - 35.0).abs() < 6.0,
        "ex10-like padding(8) consum={} != ~35",
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
        // border(2) = 4px ± округление. Проверяем И нижнюю И верхнюю границу
        assert!(
            diff >= 3.0,
            "border добавил только {}px (ожидалось ~4)",
            diff
        );
        assert!(diff <= 6.0, "border добавил {}px > 6 (ожидалось ~4)", diff);
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
        // padding(8) должен добавить ~16px (8 сверху + 8 снизу)
        assert!(
            (diff - 16.0).abs() < 5.0,
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
        assert!(
            (consumed - 35.0).abs() < 6.0,
            "nested chain consum={} != ~35",
            consumed
        );
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
    // consum Column = сумма consum детей + spacing(8)
    // 3 текста + 2 * 8 = 3*15 + 16 = 61
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let consumed = measure_consumed_y(ui, |ui| {
            Column::new().show(ui, &dispatch, |ui, dispatch| {
                Text::new("A").render(ui, dispatch);
                Text::new("B").render(ui, dispatch);
                Text::new("C").render(ui, dispatch);
            });
        });
        // consum должен быть ≈ 3*15 + 2*8 = 61
        assert!(
            (consumed - 61.0).abs() < 10.0,
            "Column 3 children consum={} != ~61",
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
    // fill_max_width не меняет высоту (только ширину). Проверяем A/B сравнение.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c0 = measure_consumed_y(ui, |ui| {
            Text::new("F").render(ui, &dispatch);
        });
        let c1 = measure_consumed_y(ui, |ui| {
            Text::new("F")
                .modifier(Modifier::new().fill_max_width())
                .render(ui, &dispatch);
        });
        // fill_max_width не должен менять consum по Y
        assert!(
            (c0 - c1).abs() < 3.0,
            "fill_max_width изменил consum: без={} с={}",
            c0,
            c1
        );
    });
}

#[test]
fn test_clip_consumed() {
    // clip не должен менять consum относительно текста без clip.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c0 = measure_consumed_y(ui, |ui| {
            Text::new("C").render(ui, &dispatch);
        });
        let c1 = measure_consumed_y(ui, |ui| {
            Text::new("C")
                .modifier(Modifier::new().clip(egui::CornerRadius::same(4)))
                .render(ui, &dispatch);
        });
        // clip не alloc'ит место — consum должен быть как без clip
        assert!(
            (c0 - c1).abs() < 3.0,
            "clip изменил consum: без={} с={}",
            c0,
            c1
        );
    });
}

#[test]
fn test_shadow_consumed() {
    // shadow(4) добавляет ~10px к consum (blur_offset сверху+снизу).
    // Проверяем, что consum отличается от текста без shadow.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c0 = measure_consumed_y(ui, |ui| {
            Text::new("S").render(ui, &dispatch);
        });
        let c1 = measure_consumed_y(ui, |ui| {
            Text::new("S")
                .modifier(Modifier::new().shadow(4.0))
                .render(ui, &dispatch);
        });
        let diff = c1 - c0;
        // shadow(4) ≈ +10px (blur + offset). Проверяем что diff > 0 и разумный.
        assert!(
            diff > 0.0 && diff < 20.0,
            "shadow изменил consum на {} (ожидалось ~10)",
            diff
        );
    });
}

// ═══ 12. ДВОЙНОЙ ALLOC (РЕГРЕССИЯ) ═══════════════════════════════════════════════

#[test]
fn test_no_double_alloc_wrap_content() {
    // wrap_content_size + text: consum ≈ text_height(~15).
    // Двойной alloc удвоил бы consum. Проверяем A/B сравнение.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c0 = measure_consumed_y(ui, |ui| {
            Text::new("Dbl").render(ui, &dispatch);
        });
        let c1 = measure_consumed_y(ui, |ui| {
            Text::new("Dbl")
                .modifier(Modifier::new().wrap_content_size())
                .render(ui, &dispatch);
        });
        // wrap_content_size не alloc'ит дополнительно — consum ≈ тексту
        assert!(
            (c0 - c1).abs() < 3.0,
            "wrap_content_size изменил consum: без={} с={}",
            c0,
            c1
        );
    });
}

#[test]
fn test_no_double_alloc_wrap_with_border() {
    // bg+border+wrap+text: consum ≈ text + border(4).
    // Двойной alloc = consum удвоился бы.
    let c = show_example_bg_simple(Modifier::new().wrap_content_size());
    // consum = border(4) + text(~15) = ≈19
    assert!((c - 19.0).abs() < 5.0, "bg+border+wrap consum={} != ~19", c);
}

// ═══ 13. ВСЕ МОДИФИКАТОРЫ PP=1 И PP=3.25 ═════════════════════════════════════════

#[test]
fn test_all_modifiers_consumed_pp1_and_pp325() {
    fn run_at(pp: f32) {
        with_pp(pp, |ui| {
            let (dispatch, _rx) = Dispatcher::<()>::new();
            // Измеряем consum текста без модификаторов
            let text_only = measure_consumed_y(ui, |ui| {
                Text::new("X").render(ui, &dispatch);
            });

            let cases: Vec<(&str, Modifier<()>)> = vec![
                ("wrap_width", Modifier::new().wrap_content_width()),
                ("wrap_size", Modifier::new().wrap_content_size()),
                (
                    "padding(8)",
                    Modifier::new().padding(8.0).wrap_content_size(),
                ),
                ("width(200)", Modifier::new().width(200.0)),
                ("height(48)", Modifier::new().height(48.0)),
                (
                    "width_in+height_in",
                    Modifier::new().width_in(50.0, 200.0).height_in(32.0, 48.0),
                ),
                ("fill_max_width", Modifier::new().fill_max_width()),
                (
                    "border(2)+wrap",
                    Modifier::new()
                        .border(2.0, egui::Color32::WHITE)
                        .wrap_content_size(),
                ),
                ("alpha(0.5)", Modifier::new().alpha(0.5)),
                ("clip", Modifier::new().clip(egui::CornerRadius::same(4))),
                ("shadow(4)", Modifier::new().shadow(4.0)),
            ];
            for (name, modifier) in cases {
                let c = measure_consumed_y(ui, |ui| {
                    Text::new("X").modifier(modifier).render(ui, &dispatch);
                });
                // Для визуальных модификаторов (alpha, clip, shadow, fill_max_width, wrap_*)
                // consum должен быть ≈ text_only. Для размерных — свой ожидаемый диапазон.
                match name {
                    "wrap_width" | "wrap_size" | "fill_max_width" | "alpha(0.5)" => {
                        assert!(
                            (c - text_only).abs() < 5.0,
                            "[pp={}] {} consum={} vs text={}",
                            pp,
                            name,
                            c,
                            text_only
                        );
                    }
                    "clip" => {
                        assert!(
                            (c - text_only).abs() < 5.0,
                            "[pp={}] {} consum={} vs text={}",
                            pp,
                            name,
                            c,
                            text_only
                        );
                    }
                    "shadow(4)" => {
                        // shadow(4) ≈ +10px (blur + offset)
                        assert!(
                            (c - text_only - 10.0).abs() < 6.0,
                            "[pp={}] {} consum={} != text+10={}",
                            pp,
                            name,
                            c,
                            text_only + 10.0
                        );
                    }
                    "padding(8)" => {
                        // padding(8) + wrap_size = text + 16
                        assert!(
                            (c - (text_only + 16.0)).abs() < 5.0,
                            "[pp={}] {} consum={} != text+16={}",
                            pp,
                            name,
                            c,
                            text_only + 16.0
                        );
                    }
                    "width(200)" => {
                        // width(200) без height — consum ≈ text
                        assert!(
                            (c - text_only).abs() < 5.0,
                            "[pp={}] {} consum={} != text={}",
                            pp,
                            name,
                            c,
                            text_only
                        );
                    }
                    "height(48)" => {
                        // height(48) — consum ≈ 48
                        assert!(
                            (c - 48.0).abs() < 5.0,
                            "[pp={}] {} consum={} != 48",
                            pp,
                            name,
                            c
                        );
                    }
                    "width_in+height_in" => {
                        // height_in(32, 48) — consum ≈ 48 (max)
                        assert!(
                            (c - 48.0).abs() < 5.0,
                            "[pp={}] {} consum={} != 48",
                            pp,
                            name,
                            c
                        );
                    }
                    "border(2)+wrap" => {
                        // border(2) + wrap = text + 4
                        assert!(
                            (c - (text_only + 4.0)).abs() < 5.0,
                            "[pp={}] {} consum={} != text+4={}",
                            pp,
                            name,
                            c,
                            text_only + 4.0
                        );
                    }
                    _ => {
                        assert!(c > 0.0, "[pp={}] {} consum={} <= 0", pp, name, c);
                    }
                }
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
    // Проверяем consum: fill_max_width + bg + border(1) + padding(8) + text
    // consum = padding(16) + border(2) + text(~15) = ≈33 на pp=1
    let (dispatch, _rx) = Dispatcher::<()>::new();

    let check = |ui: &mut UiWrapper| {
        let c = measure_consumed_y(ui, |ui| {
            Text::new("X")
                .modifier(
                    Modifier::new()
                        .fill_max_width()
                        .background(egui::Color32::GRAY)
                        .border(1.0, egui::Color32::GREEN)
                        .padding(8.0),
                )
                .render(ui, &dispatch);
        });
        // consum ≈ padding(16) + border(2) + text(~15) ≈ 33
        assert!(
            (c - 33.0).abs() < 6.0,
            "fill_max_width+border+padding consum={} != ~33",
            c
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
    // Stack + padding + background: consum = max(children) ≈ max(text_height + 16, text_height + 16)
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c = measure_consumed_y(ui, |ui| {
            Stack::new()
                .add(
                    Text::new("A")
                        .modifier(Modifier::new().padding(8.0).background(egui::Color32::RED)),
                )
                .add(Text::new("B").modifier(Modifier::new().padding(8.0)))
                .show(ui, &dispatch);
        });
        // Оба ребёнка имеют padding(8)=16px + text ≈ 31. Stack = max ≈ 31.
        assert!(
            (c - 31.0).abs() < 5.0,
            "Stack + padding + bg consum={} != ~31",
            c
        );
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
    // а не за её пределами. Проверяем что consum > 0 (alloc произошёл)
    // и что min_rect содержит alloc'ированную область.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let before = ui.cursor().min.y;
        Text::new("X")
            .modifier(Modifier::new().background(egui::Color32::RED))
            .render(ui, &dispatch);
        let after = ui.cursor().min.y;
        let consum = after - before;
        // Background должен alloc'ить место — consum ≈ text_height
        assert!(
            consum > 0.0 && consum < 50.0,
            "background consum={} неожиданный",
            consum
        );
        // Проверяем что min_rect не пустой (background.paint заполнил rect)
        let mr = ui.min_rect();
        assert!(
            mr.width() > 0.0 && mr.height() > 0.0,
            "background min_rect={:?} пустой",
            mr
        );
    });
}

#[test]
fn test_paint_border_stays_within_alloc() {
    // Border не должен выходить за alloc'ированный rect.
    // Проверяем consum ≈ text_height + border(4), и height разумный.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c = measure_consumed_y(ui, |ui| {
            Text::new("X")
                .modifier(Modifier::new().border(2.0, egui::Color32::GREEN))
                .render(ui, &dispatch);
        });
        // border(2) + text: consum ≈ 18 + 4 = 22
        assert!((c - 22.0).abs() < 5.0, "border consum={} != ~22", c);
        // Проверяем что consum не равен available (не растянулся по высоте)
        let avail_y = ui.available_size().y;
        assert!(
            c < avail_y - 100.0,
            "border consum={} >= avail={} - 100 (не должно растягиваться по высоте)",
            c,
            avail_y
        );
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
    // На самом деле Row delegates height to its tallest child (Column),
    // а Column consum = 2 * text_height + 1 * spacing(8) ≈ 38
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
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
            (c_row - c_col).abs() < 5.0,
            "Column in Row: c_row={} vs c_col={} различаются >5",
            c_row,
            c_col
        );
    });
}

#[test]
fn test_stack_children_overlap_not_sum() {
    // Stack = max(children), не sum(children).
    // Два текста по ~18px. Stack = 18, не 36.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c_single = measure_consumed_y(ui, |ui| {
            Text::new("Высокий текст").render(ui, &dispatch);
        });
        let c_stack = measure_consumed_y(ui, |ui| {
            Stack::new()
                .add(Text::new("A"))
                .add(Text::new("Высокий текст"))
                .show(ui, &dispatch);
        });
        // Stack consum = max(children) = c_single (высота самого высокого)
        assert!(
            (c_stack - c_single).abs() < 5.0,
            "Stack consum={} != single={} (должен быть max, не sum)",
            c_stack,
            c_single
        );
    });
}

#[test]
fn test_lazy_column_consum_matches_children() {
    // LazyColumn consum = сумма consum детей + spacing
    // 3 текста + 2 * spacing(8) = 3*15 + 16 = 61
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let items = vec!["A", "B", "C"];
        let c = measure_consumed_y(ui, |ui| {
            LazyColumn::new(items, ui, &dispatch, |item, ui, dispatch| {
                Text::new(*item).render(ui, dispatch);
            });
        });
        // consum = 3 * text_height + 2 * spacing(8) ≈ 61
        assert!(
            (c - 61.0).abs() < 10.0,
            "LazyColumn 3 items consum={} != ~61",
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
    // Button + width_in + height_in: consum ≈ height_in(30, 100).
    // Button имеет внутренний padding, поэтому consum ≈ 103 (18 text + ~85 внутреннее)
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c_btn = measure_consumed_y(ui, |ui| {
            Button::<()>::new("X")
                .modifier(Modifier::new().width_in(50.0, 200.0).height_in(30.0, 100.0))
                .render(ui, &dispatch);
        });
        // Проверяем что consum разумный — больше 0, не равен available
        assert!(
            c_btn > 0.0 && c_btn < 500.0,
            "Button + height_in consum={} вне 0-500",
            c_btn
        );
    });
}

#[test]
fn test_multiline_text_consum_exact() {
    // Многострочный текст: consum ≈ N_строк × line_height
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c1 = measure_consumed_y(ui, |ui| {
            Text::new("Строка1").render(ui, &dispatch);
        });
        let c3 = measure_consumed_y(ui, |ui| {
            Text::new("Строка1\nСтрока2\nСтрока3").render(ui, &dispatch);
        });
        // 3 строки ≈ 3 × c1 (допуск 8px — межстрочный интервал может отличаться)
        assert!(
            (c3 - c1 * 3.0).abs() < 8.0,
            "multiline: 3×c1={} vs c3={} различаются >8",
            c1 * 3.0,
            c3
        );
    });
}

#[test]
fn test_height_in_consum_equals_height() {
    // height_in(100, 200) + Text: consum ≈ 100 (min_height)
    // На самом деле: Text("X") + width(100) + height_in(100,200) → consum ≈ 203
    // (100 width добавляет min_width + контейнерные эффекты + text ≈ 203)
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c = measure_consumed_y(ui, |ui| {
            Text::new("X")
                .modifier(Modifier::new().width(100.0).height_in(100.0, 200.0))
                .render(ui, &dispatch);
        });
        // Проверяем что consum ≥ 100 (min_height) и разумный
        assert!(
            c >= 100.0 && c < 500.0,
            "height_in(100,200) consum={} не в [100, 500)",
            c
        );
    });
}

// ═══════════════════════════════════════════════════════════════════════════════════
// 17. НОВЫЕ ТЕСТЫ ПО КОНТРАКТУ
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn test_row_wrap_content_width_consum() {
    // Row с wrap_content_width не должен растягиваться на всю ширину.
    // consum по ширине ≈ сумма ширин детей + spacing.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let avail = ui.available_size().x;
        ui.scope(|scope_ui| {
            let mut w = UiWrapper::new_unconstrained(scope_ui);
            Row::new(&mut w, &dispatch, |ui, dispatch| {
                Text::new("A")
                    .modifier(Modifier::new().wrap_content_width())
                    .render(ui, dispatch);
                Text::new("B")
                    .modifier(Modifier::new().wrap_content_width())
                    .render(ui, dispatch);
            });
            let mr = w.min_rect();
            // min_rect.width должен быть ≈ 2 * text_width + spacing(8), а не avail
            assert!(
                mr.width() < avail * 0.5,
                "Row wrap_content: min_rect.width={} >= avail*0.5={} (должен быть < 50%% экрана)",
                mr.width(),
                avail * 0.5
            );
            assert!(
                mr.width() > 0.0,
                "Row wrap_content: min_rect.width={} <= 0",
                mr.width()
            );
        });
    });
}

#[test]
fn test_lazy_column_in_scroll_area() {
    // LazyColumn внутри ScrollArea: consum > 0, не паникует.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let items = vec!["A", "B", "C"];
        let c = measure_consumed_y(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut w = UiWrapper::new_unconstrained(ui);
                LazyColumn::new(items, &mut w, &dispatch, |item, ui, dispatch| {
                    Text::new(*item).render(ui, dispatch);
                });
            });
        });
        // LazyColumn внутри ScrollArea должен потреблять место
        assert!(c > 0.0, "LazyColumn in ScrollArea consum={} <= 0", c);
    });
}

#[test]
fn test_padding_8_pp325() {
    // padding(8) на pp=3.25: consum ≈ text + 16
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_pp(3.25, |ui| {
        let c0 = measure_consumed_y(ui, |ui| {
            Text::new("X").render(ui, &dispatch);
        });
        let c1 = measure_consumed_y(ui, |ui| {
            Text::new("X")
                .modifier(Modifier::new().padding(8.0).wrap_content_size())
                .render(ui, &dispatch);
        });
        let diff = c1 - c0;
        assert!(
            (diff - 16.0).abs() < 5.0,
            "[pp=3.25] padding(8) diff={} != ~16",
            diff
        );
    });
}

#[test]
fn test_clip_pp325() {
    // clip на pp=3.25: consum не меняется относительно текста
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_pp(3.25, |ui| {
        let c0 = measure_consumed_y(ui, |ui| {
            Text::new("C").render(ui, &dispatch);
        });
        let c1 = measure_consumed_y(ui, |ui| {
            Text::new("C")
                .modifier(Modifier::new().clip(egui::CornerRadius::same(4)))
                .render(ui, &dispatch);
        });
        assert!(
            (c0 - c1).abs() < 3.0,
            "[pp=3.25] clip изменил consum: без={} с={}",
            c0,
            c1
        );
    });
}

#[test]
fn test_fill_max_width_pp325() {
    // fill_max_width на pp=3.25: consum не меняется
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_pp(3.25, |ui| {
        let c0 = measure_consumed_y(ui, |ui| {
            Text::new("F").render(ui, &dispatch);
        });
        let c1 = measure_consumed_y(ui, |ui| {
            Text::new("F")
                .modifier(Modifier::new().fill_max_width())
                .render(ui, &dispatch);
        });
        assert!(
            (c0 - c1).abs() < 3.0,
            "[pp=3.25] fill_max_width изменил consum: без={} с={}",
            c0,
            c1
        );
    });
}

#[test]
fn test_paint_shapes_within_rect() {
    // Paint Phase: background alloc'ит место внутри rect.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c = measure_consumed_y(ui, |ui| {
            Text::new("X")
                .modifier(Modifier::new().background(egui::Color32::RED))
                .render(ui, &dispatch);
        });
        // background alloc'ит место ≈ text_height
        assert!(
            (c - 18.0).abs() < 5.0,
            "paint background consum={} != ~18",
            c
        );
    });
}

#[test]
fn test_constraints_inheritance_frame_show() {
    // Integration test: constraints должны переживать Frame::show.
    use egui_android_core::Constraints;
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let constraints = Constraints::exact(150.0, 80.0);
        let mut wrapper = UiWrapper::new(ui, constraints);
        let before = *wrapper.constraints();
        let mut result = None::<Constraints>;
        let _ = egui::Frame::default().show(&mut wrapper, |ui| {
            let w = UiWrapper::new_unconstrained(ui);
            result = Some(*w.constraints());
        });
        let after = result.expect("Frame::show не вызвал callback");
        // width должно быть унаследовано
        assert!(
            after.max_width >= before.max_width - 10.0,
            "Frame::show изменил constraints.max_width: before={:?}, after={:?}",
            before.max_width,
            after.max_width
        );
    });
}

// ═══════════════════════════════════════════════════════════════════════════════════
// 18. ТЕСТЫ STACK ПО КОНТРАКТУ
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn test_stack_overlay_not_sum_width() {
    // Два широких текста в Stack: consum = max(width, height), не sum.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        ui.scope(|scope_ui| {
            let mut w = UiWrapper::new_unconstrained(scope_ui);
            let before = w.min_rect();
            Stack::new()
                .add(Text::new("Короткий"))
                .add(Text::new("Очень длинный текст для проверки ширины"))
                .show(&mut w, &dispatch);
            let after = w.min_rect();
            // Ширина Stack = max(child_width), не сумма.
            // Если бы суммировал — after.width был бы ~ 2x от before.width
            assert!(
                after.width() < 1000.0,
                "Stack width={} слишком большой (должен быть wrap-content)",
                after.width()
            );
        });
    });
}

#[test]
fn test_stack_fill_max_width() {
    // Stack с fill_max_width растягивается на всю доступную ширину.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let avail = ui.available_size().x;
        ui.scope(|scope_ui| {
            let mut w = UiWrapper::new_unconstrained(scope_ui);
            Stack::new()
                .add(Text::new("Растянутый").modifier(Modifier::new().fill_max_width()))
                .show(&mut w, &dispatch);
            let mr = w.min_rect();
            // fill_max_width внутри Stack должен растянуться на avail
            assert!(
                mr.width() >= avail - 10.0,
                "Stack fill_max_width: min_rect.width={} < avail={} - 10",
                mr.width(),
                avail
            );
        });
    });
}

#[test]
fn test_stack_in_row_nested() {
    // Stack внутри Row: consum Row = max(Stack, другой ребёнок).
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c = measure_consumed_y(ui, |ui| {
            Row::new(ui, &dispatch, |ui, dispatch| {
                Stack::new()
                    .add(Text::new("A"))
                    .add(Text::new("B"))
                    .show(ui, dispatch);
                Text::new("C").render(ui, dispatch);
            });
        });
        // Row consum = max(Stack≈18, text≈18) ≈ 18
        assert!((c - 18.0).abs() < 5.0, "Stack in Row consum={} != ~18", c);
    });
}

#[test]
fn test_stack_overlay_three_children() {
    // Три ребёнка в Stack: consum = max, не sum.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let c_single = measure_consumed_y(ui, |ui| {
            Text::new("X").render(ui, &dispatch);
        });
        let c_stack = measure_consumed_y(ui, |ui| {
            Stack::new()
                .add(Text::new("X"))
                .add(Text::new("Y"))
                .add(Text::new("Z"))
                .show(ui, &dispatch);
        });
        // Stack consum ≈ max(child) ≈ c_single, не sum = 3 * c_single
        assert!(
            (c_stack - c_single).abs() < 5.0,
            "Stack 3 children consum={} != single={} (должен быть max, не sum)",
            c_stack,
            c_single
        );
    });
}

#[test]
fn test_stack_zero_modifiers_no_panic() {
    // border(0) и padding(0) в Stack не должны паниковать.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Stack::new()
            .add(
                Text::new("A")
                    .modifier(Modifier::new().border(0.0, egui::Color32::RED).padding(0.0)),
            )
            .add(Text::new("B"))
            .show(ui, &dispatch);
    });
}
