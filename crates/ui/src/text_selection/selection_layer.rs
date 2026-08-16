//! Selection-слой поверх опубликованной текстовой поверхности.
//!
//! Используется модификатором `Modifier::selectable(true)`: рисует выделение
//! (фон-прямоугольники, ручки, тулбар) ПОВЕРХ уже отрисованного текста, не
//! перерисовывая galley. Читает `TextSurface`, опубликованную виджетом.

use egui::text::CCursorRange;
use egui::vec2;
use egui_android_core::UiWrapper;

use super::android_behavior::select_word_at;
use super::drag_handles::{dragged_handle, draw_handle, handle_positions, selection_bbox};
use super::selection_toolbar::{show_toolbar, ToolbarAction};
use super::surface::TextSurface;
use super::{AndroidSelectionState, HandleSide, LongPressState};
use crate::remember::remember;

/// Обработать выделение поверх текстовой поверхности.
///
/// Pointer берётся НЕ из `Response`-интеракции (которая ненадёжна на этом
/// Android-бэкенде и не совпадает с пальцем при скролле/перекрытии), а напрямую
/// из `ui.input(|i| i.pointer...)` — глобальный pointer-статус. Если палец в
/// границах текста (`surface` → `text_rect`) — считаем его активным для выделения.
pub(crate) fn show_selection_layer(ui: &mut UiWrapper, surface: &TextSurface) {
    // Per-widget состояние: ключ — id из опубликованной поверхности (уникален
    // на виджет, не зависит от `ui.id()` который общий для нескольких selectable).
    let state_key = surface.id;
    let lp = remember(ui, ("sel_lp", state_key), LongPressState::default);
    let sel = remember(ui, ("sel_state", state_key), AndroidSelectionState::default);

    let galley = &surface.galley;
    let galley_pos = surface.galley_pos;
    let text_rect = egui::Rect::from_min_size(galley_pos, galley.size());
    // Небольшой запас для касания по краю строки.
    let hit_rect = text_rect.expand(16.0);

    // ── Pointer-статус (глобальный, надёжный) ──
    //
    // Логика: пока палец нажат ЛЮБОЙ точкой экрана и в момент касания палец был
    // внутри текста (`press_pos` запомнен) — считаем удержание активным. Это
    // защищает от сброса таймера, когда палец чуть «дрожит» на границе rect
    // между кадрами. Смещение > MAX_PRESS_DRIFT (скролл/дрейф) всё же отменяет.
    let (now, any_down, latest) =
        ui.input(|i| (i.time, i.pointer.any_down(), i.pointer.latest_pos()));

    let mut lp_state = lp.get().clone();
    let in_text = latest.map_or(false, |p| hit_rect.contains(p));
    let start_in_text = lp_state.press_pos.is_none() && any_down && in_text;
    let active_press = lp_state.press_pos.is_some() || start_in_text;
    let down = any_down && active_press;
    let recognized = lp_state.update_raw(now, down, latest);
    if recognized {
        log::info!("SEL-LAYER: LONG-PRESS распознан (first-time diagnostic)");
    }
    lp.set(lp_state);

    let mut sel_state = sel.get().clone();

    // Тап вне текста (был нажат в прошлом, теперь отпущен вне) → сброс.
    // Для простоты: если палец отпущен и не был в тексте — сбрасываем.
    if !any_down && sel_state.active && !in_text {
        sel_state.active = false;
        sel_state.selection = None;
        sel_state.selection_rect = None;
    } else if recognized {
        if let Some(pos) = latest {
            let local = pos - galley_pos.to_vec2();
            let cursor = galley.cursor_from_pos(local.to_vec2());
            let range = select_word_at(&galley.text(), cursor);
            let selected = range.slice_str(&galley).to_owned();
            sel_state.active = !range.is_empty() && !selected.is_empty();
            if sel_state.active {
                sel_state.selection = Some(range);
                sel_state.selected_text = selected;
                sel_state.selection_rect = Some(selection_bbox(galley, galley_pos, &range));
            } else {
                sel_state.selection = None;
                sel_state.selection_rect = None;
            }
        }
    }

    // ── Рендер выделения поверх текста ──
    let range = sel_state.selection;

    if let Some(range) = range.filter(|r| !r.is_empty()) {
        log::info!(
            "SEL: render active={} range={:?} sel_rect={:?} text={:?}",
            sel_state.active,
            range.as_sorted_char_range(),
            sel_state.selection_rect,
            sel_state.selected_text.chars().take(12).collect::<String>()
        );
        // Фон выделения — полупрозрачные прямоугольники по строкам.
        paint_selection_background(ui, surface, &range);

        // Ручки на границах выделения.
        let (sp, ep) = handle_positions(galley, galley_pos, &range);
        let accent = ui.visuals().selection.stroke.color;
        log::info!("SEL: handles sp={:?} ep={:?}", sp, ep);
        let s_resp = draw_handle(ui, state_key.with("sel_handle_s"), sp, accent);
        let e_resp = draw_handle(ui, state_key.with("sel_handle_e"), ep, accent);

        // Drag ручки → расширение/сужение диапазона.
        if let Some(handle) = dragged_handle(&s_resp, &e_resp) {
            if let Some(pos) = latest {
                let local = pos - galley_pos.to_vec2();
                let cursor = galley.cursor_from_pos(local.to_vec2());
                if let Some(range) = sel_state.selection.as_mut() {
                    match handle {
                        HandleSide::Start => {
                            range.secondary = cursor;
                        }
                        HandleSide::End => {
                            range.primary = cursor;
                        }
                    }
                    sel_state.selected_text = range.slice_str(galley).to_owned();
                    sel_state.selection_rect = Some(selection_bbox(galley, galley_pos, range));
                }
            }
        }

        // Тулбар над выделением.
        if let Some(bbox) = sel_state.selection_rect {
            log::info!("SEL: show_toolbar bbox={:?}", bbox);
            if let Some(action) = show_toolbar(ui.ctx(), state_key.with("sel_toolbar"), bbox) {
                log::info!("SEL: toolbar action={:?}", action);
                match action {
                    ToolbarAction::Copy => {
                        ui.copy_text(sel_state.selected_text.clone());
                        sel_state.active = false;
                        sel_state.selection = None;
                        sel_state.selection_rect = None;
                    }
                    ToolbarAction::SelectAll => {
                        let all = CCursorRange::select_all(galley);
                        sel_state.selection = Some(all);
                        sel_state.selected_text = galley.text().to_owned();
                        sel_state.selection_rect = Some(selection_bbox(galley, galley_pos, &all));
                    }
                    _ => {}
                }
            }
        }
    } else {
        log::info!("SEL: no selection to render (active={})", sel_state.active);
    }

    sel.set(sel_state);
}

/// Нарисовать фоновые прямоугольники выделения по строкам галели поверх текста.
fn paint_selection_background(ui: &mut UiWrapper, surface: &TextSurface, range: &CCursorRange) {
    if range.is_empty() {
        return;
    }
    let galley = &surface.galley;
    let galley_pos = surface.galley_pos;
    let bg = ui.visuals().selection.bg_fill;

    let [min, max] = range.sorted_cursors();
    let min_layout = galley.layout_from_cursor(min);
    let max_layout = galley.layout_from_cursor(max);

    let painter = ui.painter();
    let mut painted = 0;
    for ri in min_layout.row..=max_layout.row {
        if let Some(placed_row) = galley.rows.get(ri) {
            let row = &placed_row.row;
            let left = if ri == min_layout.row {
                row.x_offset(min_layout.column)
            } else {
                0.0
            };
            let right = if ri == max_layout.row {
                row.x_offset(max_layout.column)
            } else {
                row.size.x
            };
            // Глобальная позиция строки.
            let row_min = galley_pos + vec2(left, placed_row.pos.y);
            let row_size = vec2(right - left, placed_row.row.size.y);
            painter.rect_filled(egui::Rect::from_min_size(row_min, row_size), 0.0, bg);
            painted += 1;
            if painted == 1 {
                log::info!(
                    "SEL: paint_selection_background row0 rect=[{:?}] rows={}",
                    egui::Rect::from_min_size(row_min, row_size),
                    galley.rows.len()
                );
            }
        }
    }
}
