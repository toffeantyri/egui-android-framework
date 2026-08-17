//! Drag-ручки (капельки Android) для границ выделения.
//!
//! Рисуются поверх галели в touch-ветке `Text`. Чистая логика координат —
//! в `handle_positions`; визуал — `draw_handle`.

use egui::text::CCursorRange;
use egui::{Color32, Id, Order, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};

/// Радиус капельки (точек).
const HANDLE_RADIUS: f32 = 12.0;
/// Ширина стебля.
const STEM_WIDTH: f32 = 2.0;
/// Высота стебля (от текста до капельки).
const STEM_HEIGHT: f32 = 24.0;
/// Зона захвата (увеличенная для пальца).
const HIT_AREA: f32 = 24.0;

/// Позиции двух ручек из `CCursorRange` — в координатах, согласованных с галели.
///
/// Возвращает `(start, end)` — точки привязки стеблей (на границе строки).
pub(crate) fn handle_positions(
    galley: &egui::Galley,
    galley_pos: Pos2,
    range: &CCursorRange,
) -> (Pos2, Pos2) {
    let [min, max] = range.sorted_cursors();
    let start = galley_pos + galley.pos_from_cursor(min).center().to_vec2();
    let end = galley_pos + galley.pos_from_cursor(max).center().to_vec2();
    (start, end)
}

/// Грубый bounding-box выделения в координатах экрана (для позиционирования тулбара).
pub(crate) fn selection_bbox(
    galley: &egui::Galley,
    galley_pos: Pos2,
    range: &CCursorRange,
) -> Rect {
    let [min, max] = range.sorted_cursors();
    let min_rect = galley.pos_from_cursor(min).translate(galley_pos.to_vec2());
    let max_rect = galley.pos_from_cursor(max).translate(galley_pos.to_vec2());
    min_rect.union(max_rect)
}

/// Какая ручка сейчас перетаскивается (для drag-обновления диапазона).
pub(crate) fn dragged_handle(
    start_resp: &Response,
    end_resp: &Response,
) -> Option<super::android_behavior::HandleSide> {
    if start_resp.dragged() {
        Some(super::android_behavior::HandleSide::Start)
    } else if end_resp.dragged() {
        Some(super::android_behavior::HandleSide::End)
    } else {
        None
    }
}

/// Нарисовать одну ручку выделения. Возвращает Response для drag.
///
/// ИСПРАВЛЕНИЕ (этап 5): убрана проверка `ui.is_rect_visible(hit_rect)`, которая
/// прятала ручки вне clip-области текста. Ручки рисуются всегда; ограничение
/// видимости обеспечивается через `Area` на уровне вызывающего кода.
pub(crate) fn draw_handle(ui: &mut Ui, id: Id, anchor_pos: Pos2, color: Color32) -> Response {
    // Стебель висит под строкой текста.
    let stem_bottom = anchor_pos + Vec2::new(0.0, STEM_HEIGHT);
    let drop_center = stem_bottom + Vec2::new(0.0, HANDLE_RADIUS * 0.7);

    // Невидимая (увеличенная) зона захвата для пальца.
    let hit_rect = Rect::from_center_size(drop_center, Vec2::splat(HIT_AREA));
    let response = ui.interact(hit_rect, id, Sense::drag());

    let painter = ui.painter();

    // Стебель.
    painter.line_segment([anchor_pos, stem_bottom], (STEM_WIDTH, color));
    // Капелька.
    painter.circle_filled(drop_center, HANDLE_RADIUS * 0.7, color);
    // Обводка при перетаскивании.
    if response.dragged() {
        painter.circle_stroke(
            drop_center,
            HANDLE_RADIUS * 0.7 + 2.0,
            Stroke::new(1.5, color),
        );
    }

    response
}

/// Нарисовать обе ручки через `Area` в `Order::Foreground`.
///
/// Обходит ограничение clip-зоны текста: даже за пределами `clip_rect` ручки
/// остаются видимыми. Возвращает `(Response для start, Response для end)`.
pub(crate) fn draw_handles_in_area(
    ctx: &egui::Context,
    id: Id,
    start_pos: Pos2,
    end_pos: Pos2,
    color: Color32,
) -> (Option<Response>, Option<Response>) {
    let mut s_resp: Option<Response> = None;
    let mut e_resp: Option<Response> = None;

    egui::Area::new(id.with("sel_handles_area"))
        .order(Order::Foreground)
        .fixed_pos(egui::Pos2::ZERO)
        .constrain_to(egui::Rect::EVERYTHING)
        .show(ctx, |area_ui| {
            s_resp = Some(draw_handle(area_ui, id.with("h_s"), start_pos, color));
            e_resp = Some(draw_handle(area_ui, id.with("h_e"), end_pos, color));
        });

    (s_resp, e_resp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::text::CCursor;

    /// Запустить замыкание с реальным egui-ui (шрифты загружены,
    /// в отличие от `__run_test_ui`, который ставит пустые шрифты).
    fn with_real_ui(f: impl FnOnce(&egui::Ui)) {
        let f = std::cell::RefCell::new(Some(f));
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let f = f.borrow_mut().take().unwrap();
                f(ui);
            });
        });
    }

    fn make_job(text: &str) -> egui::text::LayoutJob {
        egui::text::LayoutJob {
            text: text.to_owned(),
            sections: vec![egui::text::LayoutSection {
                leading_space: 0.0,
                byte_range: egui::text::ByteIndex(0)..egui::text::ByteIndex(text.len()),
                format: egui::text::TextFormat {
                    font_id: egui::FontId::proportional(14.0),
                    ..Default::default()
                },
            }],
            wrap: egui::text::TextWrapping {
                max_width: 1000.0,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn handle_positions_from_range_ordered() {
        with_real_ui(|ui| {
            let galley = ui.painter().layout_job(make_job("hello"));
            assert!(
                galley.size().x > 0.0,
                "галель должна быть непустой (size={:?})",
                galley.size()
            );
            let range = CCursorRange::two(CCursor::new(0), CCursor::new(5));
            let galley_pos = Pos2::ZERO;

            let (start, end) = handle_positions(&galley, galley_pos, &range);
            // Одна строка → start.y == end.y; start.x < end.x.
            assert!(
                start.x < end.x,
                "start.x={} должен быть < end.x={}",
                start.x,
                end.x
            );
            assert!(
                (start.y - end.y).abs() < 0.01,
                "одна строка: start.y ≈ end.y"
            );
        });
    }

    #[test]
    fn handle_positions_zero_length_is_same_point() {
        with_real_ui(|ui| {
            let galley = ui.painter().layout_job(make_job("hello"));
            let range = CCursorRange::one(CCursor::new(2));
            let galley_pos = Pos2::ZERO;

            let (start, end) = handle_positions(&galley, galley_pos, &range);
            assert!(
                (start.x - end.x).abs() < 0.01 && (start.y - end.y).abs() < 0.01,
                "нулевое выделение → ручки совпадают: start={start:?} end={end:?}"
            );
        });
    }

    #[test]
    fn handle_positions_respect_galley_pos_offset() {
        with_real_ui(|ui| {
            let galley = ui.painter().layout_job(make_job("hello"));
            let range = CCursorRange::two(CCursor::new(0), CCursor::new(5));
            let galley_pos = Pos2::new(100.0, 50.0);

            let (start, end) = handle_positions(&galley, galley_pos, &range);
            // Сдвиг на galley_pos прибавляется к относительным координатам.
            assert!(
                start.x >= 100.0 && end.x >= 100.0,
                "ручки смещены на galley_pos"
            );
            // y: разница между двумя galley_pos.y = разница y ручек (+30).
            let (start_b, _) = handle_positions(&galley, Pos2::new(100.0, 20.0), &range);
            assert!(
                (start.y - start_b.y - 30.0).abs() < 0.01,
                "y следует за galley_pos.y (start.y={}, start_b.y={})",
                start.y,
                start_b.y
            );
        });
    }

    /// `draw_handles_in_area` рисует ручки поверх (Area, Foreground) без паники,
    /// даже если позиция вне обычного clip-прямоугольника (исправление этапа 5).
    #[test]
    fn draw_handles_in_area_renders_without_panic() {
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |_ui| {});
        });
        // Координаты могут быть глубоко под/влево от области — ручки всё равно рисуются.
        let (s, e) = draw_handles_in_area(
            &ctx,
            egui::Id::new("handles_test"),
            egui::pos2(-40.0, 500.0),
            egui::pos2(300.0, 500.0),
            egui::Color32::RED,
        );
        assert!(s.is_some() && e.is_some(), "Area возвращает обе ручки");
    }
}
