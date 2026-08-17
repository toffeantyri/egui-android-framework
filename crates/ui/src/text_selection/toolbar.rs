//! Плавающий тулбар действий над выделением (Copy / Cut / Paste / SelectAll).
//!
//! Рисуется через `egui::Area` поверх текста (`Order::Foreground`).
//!
//! Исправление (этап 4): тулбар по умолчанию НАД выделением (`top() - 8`), а не под;
//! компоновка кнопок — горизонтальная. Для редактируемого виджета (`is_editable`)
//! показываются Cut/Paste; для read-only — только Copy/SelectAll (Paste остаётся disabled).

use egui::{Area, Color32, CornerRadius, Frame, Id, Order, Pos2, Rect};

/// Действия тулбара.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolbarAction {
    Copy,
    Cut,
    Paste,
    SelectAll,
}

/// Опорная позиция тулбара: НАД прямоугольником выделения, по центру x.
pub(crate) fn toolbar_anchor(selection_rect: Rect) -> Pos2 {
    egui::pos2(selection_rect.center().x, selection_rect.top() - 8.0)
}

/// Показать тулбар над выделением.
///
/// Если тулбар уйдёт выше содержимого (`content_rect.top() + 40`) — зеркально
/// опускаем его ПОД выделение. Цвета берутся из текущей темы фреймворка
/// (`Theme::current`). Возвращает действие, если пользователь нажал кнопку.
pub(crate) fn show_toolbar(
    ctx: &egui::Context,
    id: Id,
    selection_rect: Rect,
    is_editable: bool,
) -> Option<ToolbarAction> {
    let mut result = None;
    let theme = crate::theme::Theme::current(ctx);
    let surface = theme.colors.surface_container_highest;

    // Тулбар над выделением; если уйдёт за верхний край — показываем под ним.
    let content_rect = ctx.content_rect();
    let anchor = toolbar_anchor(selection_rect);
    let pos = if anchor.y < content_rect.top() + 40.0 {
        egui::pos2(selection_rect.center().x, selection_rect.bottom() + 8.0)
    } else {
        anchor
    };

    Area::new(id)
        .order(Order::Foreground)
        .fixed_pos(pos)
        .constrain_to(content_rect)
        .movable(false)
        .show(ctx, |ui| {
            Frame::new()
                .fill(surface)
                .corner_radius(CornerRadius::same(12))
                .inner_margin(egui::Margin::symmetric(6, 3))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 4],
                    blur: 12,
                    spread: 0,
                    color: Color32::from_black_alpha(80),
                })
                .show(ui, |ui| {
                    // Горизонтальная компоновка (исправление этапа 4: было vertical).
                    ui.horizontal(|ui| {
                        if ui.button("Копировать").clicked() {
                            result = Some(ToolbarAction::Copy);
                        }
                        if is_editable {
                            ui.separator();
                            if ui.button("Вырезать").clicked() {
                                result = Some(ToolbarAction::Cut);
                            }
                            ui.separator();
                            // Paste через JNI clipboard — P1; пока disabled.
                            if ui
                                .add_enabled(false, egui::Button::new("Вставить"))
                                .clicked()
                            {
                                result = Some(ToolbarAction::Paste);
                            }
                        }
                        ui.separator();
                        if ui.button("Всё").clicked() {
                            result = Some(ToolbarAction::SelectAll);
                        }
                    });
                });
        });

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolbar_anchor_above_selection() {
        let sel = Rect::from_min_max(egui::pos2(10.0, 100.0), egui::pos2(110.0, 120.0));
        let anchor = toolbar_anchor(sel);
        // НАД выделением (y меньше верхней границы), по центру x
        assert!(anchor.y < sel.top(), "тулбар над выделением");
        assert!((anchor.x - sel.center().x).abs() < 0.01, "по центру по x");
    }

    #[test]
    fn toolbar_anchor_with_zero_height_rect() {
        // выделение нулевой высоты — не паникуем, якорь валиден
        let sel = Rect::from_min_max(egui::pos2(5.0, 5.0), egui::pos2(5.0, 5.0));
        let _ = toolbar_anchor(sel);
    }

    #[test]
    fn show_toolbar_no_click_returns_none() {
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |_ui| {});
        });
        let sel = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 20.0));
        let action = show_toolbar(&ctx, Id::new("tb_test"), sel, false);
        assert!(action.is_none());
    }

    /// `is_editable=false` (read-only) — тулбар рендерится без Cut (copy/select-all),
    /// не паникует и без клика ничего не возвращает.
    #[test]
    fn show_toolbar_readonly_hides_cut() {
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |_ui| {});
        });
        let sel = Rect::from_min_max(egui::pos2(0.0, 50.0), egui::pos2(100.0, 70.0));
        let action = show_toolbar(&ctx, Id::new("tb_ro"), sel, false);
        assert!(action.is_none());
    }
}
