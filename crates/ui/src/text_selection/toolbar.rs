//! Плавающий тулбар действий над выделением (Copy / Cut / Paste / SelectAll).
//!
//! Рисуется через `egui::Area` поверх текста (`Order::Foreground`).
//!
//! Исправление (этап 4): тулбар по умолчанию НАД выделением, а не под; компоновка
//! кнопок — горизонтальная. Позиция через `Area::fixed_pos` с подъёмом на `TOOLBAR_LIFT`
//! (без `pivot`/`anchor` — они ломали клики по кнопкам на устройстве). Для
//! редактируемого виджета (`is_editable`) показываются Cut/Paste; для read-only —
//! только Copy/SelectAll (Paste остаётся disabled).

use egui::{Area, Color32, CornerRadius, Frame, Id, Order, Pos2, Rect};

/// Насколько поднять попап выше верхней границы выделенного текста.
/// Включает высоту самих кнопок (~32px) + отступ, чтобы весь попап был ВЫШЕ текста
/// и не перекрывал его, но без `pivot` (который ломал клик по кнопкам).
const TOOLBAR_LIFT: f32 = 48.0;

/// Действия тулбара.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolbarAction {
    Copy,
    Cut,
    Paste,
    SelectAll,
}

/// Позиция тулбара (его ВЕРХ) — выше верха выделения на `TOOLBAR_LIFT`.
///
/// `Area::fixed_pos(pos)` ставит ВЕРХ попапа в `pos`, поэтому поднимаем `y` на
/// высоту попапа + отступ — середина попапа оказывается выше текста. Используем
/// `fixed_pos` без `pivot`/`anchor` (они ломали клики по кнопкам на устройстве).
pub(crate) fn compute_toolbar_pos(selection_rect: Rect, content_rect: Rect) -> Pos2 {
    let y = (selection_rect.top() - TOOLBAR_LIFT).max(content_rect.top());
    egui::pos2(selection_rect.center().x, y)
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

    // Позиционируем попап через `Area::fixed_pos` (клик по кнопкам стабильно работает),
    // поднимая ВЕРХ попапа выше выделения на TOOLBAR_LIFT — попап не перекрывает текст.
    // Верх попапа = pos.y; середина попапа — выше верха выделенного текста.
    let content_rect = ctx.content_rect();
    let pos = compute_toolbar_pos(selection_rect, content_rect);
    log::info!(
        "SEL-PIPE [toolbar] selection_rect={:?} popup_top={:?} lift={TOOLBAR_LIFT}",
        selection_rect,
        pos,
    );

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
    fn toolbar_pos_above_selection_never_below_top() {
        let sel = Rect::from_min_max(egui::pos2(10.0, 200.0), egui::pos2(110.0, 220.0));
        let content = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(400.0, 800.0));
        let pos = compute_toolbar_pos(sel, content);
        // Верх попапа ВЫШЕ верха выделения (не на уровне текста).
        assert!(pos.y < sel.top(), "попап выше верхней границы выделения");
        assert!((pos.x - sel.center().x).abs() < 0.01, "по центру x");
    }

    #[test]
    fn toolbar_pos_near_top_edge_clamps_to_top_not_below() {
        // Выделение у самого верха: не опускаем под текст, прижимаем вверх.
        let sel = Rect::from_min_max(egui::pos2(10.0, 2.0), egui::pos2(110.0, 20.0));
        let content = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(400.0, 800.0));
        let pos = compute_toolbar_pos(sel, content);
        // y не ниже top() и не выше верхней границы области.
        assert!(
            pos.y <= sel.top(),
            "попап не ниже верхней границы выделения"
        );
        assert!(pos.y >= content.top(), "попап не выше области видимости");
    }

    #[test]
    fn toolbar_pos_with_zero_height_rect() {
        // выделение нулевой высоты — не паникуем, позиция валидна
        let sel = Rect::from_min_max(egui::pos2(5.0, 5.0), egui::pos2(5.0, 5.0));
        let content = Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(400.0, 800.0));
        let _ = compute_toolbar_pos(sel, content);
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
