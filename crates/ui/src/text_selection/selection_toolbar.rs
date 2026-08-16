//! Плавающий тулбар действий над выделением (Copy / SelectAll / Paste-disabled).
//!
//! Рисуется через `egui::Area` поверх текста (`Order::Foreground`).

use egui::{Area, Color32, CornerRadius, Frame, Id, Order, Pos2, Rect};

/// Действия тулбара.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolbarAction {
    Copy,
    Cut,
    Paste,
    SelectAll,
}

/// Опорная позиция тулбара: ПОД прямоугольником выделения (ниже точки выделения),
/// по горизонтали — по центру.
pub(crate) fn toolbar_anchor(selection_rect: Rect) -> Pos2 {
    egui::pos2(selection_rect.center().x, selection_rect.bottom() + 8.0)
}

/// Показать тулбар под прямоугольником выделения.
///
/// Цвета берутся из текущей темы фреймворка (`Theme::current`) — фон кнопок/текста
/// соответствуют палитре, консистентно с остальными виджетами. Кнопки используют
/// стандартный `ui.button` (цвета из `ui.visuals()` темы, установленной через
/// `Theme::apply`). Возвращает действие, если пользователь нажал кнопку; `None` — иначе.
pub(crate) fn show_toolbar(
    ctx: &egui::Context,
    id: Id,
    selection_rect: Rect,
) -> Option<ToolbarAction> {
    let mut result = None;
    let theme = crate::theme::Theme::current(ctx);
    let surface = theme.colors.surface_container_highest;

    Area::new(id)
        .order(Order::Foreground)
        .fixed_pos(toolbar_anchor(selection_rect))
        .constrain_to(ctx.content_rect())
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
                    ui.vertical(|ui| {
                        if ui.button("Копировать").clicked() {
                            result = Some(ToolbarAction::Copy);
                        }
                        // Paste через JNI clipboard — P1; пока disabled.
                        if ui
                            .add_enabled(false, egui::Button::new("Вставить"))
                            .clicked()
                        {
                            result = Some(ToolbarAction::Paste);
                        }
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
    fn toolbar_anchor_below_selection() {
        let sel = Rect::from_min_max(egui::pos2(10.0, 10.0), egui::pos2(110.0, 20.0));
        let anchor = toolbar_anchor(sel);
        // ниже выделения (y больше нижней границы), по центру x
        assert!(anchor.y > sel.bottom(), "тулбар ниже выделения");
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
        let action = show_toolbar(&ctx, Id::new("tb_test"), sel);
        assert!(action.is_none());
    }
}
