//! Состояние Android-выделения для одного текстового виджета.
//!
//! Хранится per-widget через [`crate::remember::remember`]:
//! `remember(ui, ("android_sel", widget_id), AndroidSelectionState::default)`.

use super::android_behavior::HandleSide;
use egui::text::CCursorRange;

/// Полное состояние Android-выделения для одного виджета `Text`.
#[derive(Clone, Debug, Default)]
pub struct AndroidSelectionState {
    /// Активно ли выделение.
    pub active: bool,
    /// Текущий выделенный диапазон (в char-индексах галели).
    pub selection: Option<CCursorRange>,
    /// Какую ручку сейчас тянет пользователь.
    pub active_handle: Option<HandleSide>,
    /// Прямоугольник выделенной области (для позиционирования тулбара).
    pub selection_rect: Option<egui::Rect>,
    /// Выделенный текст (для копирования).
    pub selected_text: String,
}
