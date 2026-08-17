//! Рендер галели с фоновым выделением.
//!
//! В отличие от старого `paint_selection_background` (который рисовал `rect_filled`
//! ПОВЕРХ текста), здесь используется штатный `egui::text_selection::visuals::paint_text_selection`,
//! который мутирует mesh галели: фон вставляется ДО глифов (под ними).
//!
//! Функция клонирует `Arc<Galley>` перед мутацией — оригинал не затрагивается
//! (`Arc::make_mut` внутри создаст копию только на расшаренном Arc).

use egui::text::CCursorRange;
use egui::{Color32, Pos2};
use std::sync::Arc;

use egui_android_core::UiWrapper;

/// Нарисовать галель с выделением `range` (фон ПОД глифами).
///
/// Если `range` пуст — ничего не рисует (обычный рендер выполняет вызывающий код).
pub(crate) fn paint_galley_with_selection(
    ui: &mut UiWrapper,
    galley: &Arc<egui::Galley>,
    galley_pos: Pos2,
    range: &CCursorRange,
    fallback_color: Color32,
) {
    if range.is_empty() {
        return;
    }

    // Клонируем Arc и мутируем клон: `paint_text_selection` через `Arc::make_mut`
    // добавит фон в mesh, не затрагивая оригинальную галель.
    let mut galley_mut = Arc::clone(galley);
    egui::text_selection::visuals::paint_text_selection(&mut galley_mut, ui.visuals(), range, None);

    let rect = egui::Rect::from_min_size(galley_pos, galley_mut.size());
    ui.painter_at(rect)
        .galley(galley_pos, galley_mut, fallback_color);
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::text::{CCursor, CCursorRange};

    fn with_ui(f: impl FnOnce(&mut UiWrapper)) {
        let f = std::cell::RefCell::new(Some(f));
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let f = f.borrow_mut().take().unwrap();
                f(&mut UiWrapper::new_unconstrained(ui));
            });
        });
    }

    #[test]
    fn empty_range_does_not_panic_and_does_not_draw() {
        with_ui(|ui| {
            let galley: Arc<egui::Galley> = ui.painter().layout_job(egui::text::LayoutJob {
                text: "hello".into(),
                sections: vec![egui::text::LayoutSection {
                    leading_space: 0.0,
                    byte_range: egui::text::ByteIndex(0)..egui::text::ByteIndex(5),
                    format: egui::text::TextFormat {
                        font_id: egui::FontId::proportional(14.0),
                        ..Default::default()
                    },
                }],
                ..Default::default()
            });
            let empty = CCursorRange::one(CCursor::new(2));
            // не паникует, ничего не рисует
            paint_galley_with_selection(
                ui,
                &galley,
                egui::Pos2::ZERO,
                &empty,
                egui::Color32::WHITE,
            );
        });
    }

    #[test]
    fn non_empty_range_renders_without_panic() {
        with_ui(|ui| {
            let galley: Arc<egui::Galley> = ui.painter().layout_job(egui::text::LayoutJob {
                text: "hello world".into(),
                sections: vec![egui::text::LayoutSection {
                    leading_space: 0.0,
                    byte_range: egui::text::ByteIndex(0)..egui::text::ByteIndex(11),
                    format: egui::text::TextFormat {
                        font_id: egui::FontId::proportional(14.0),
                        ..Default::default()
                    },
                }],
                ..Default::default()
            });
            let range = CCursorRange::two(CCursor::new(0), CCursor::new(5));
            paint_galley_with_selection(
                ui,
                &galley,
                egui::Pos2::ZERO,
                &range,
                egui::Color32::WHITE,
            );
        });
    }
}
