//! Хелперы интерполяции значений для анимаций.
//!
//! Используют встроенные функции egui:
//! - [`egui::Context::animate_value_with_time`]
//! - [`egui::Context::animate_bool_with_time`]
//!
//! Обе функции автоматически вызывают `ctx.request_repaint()`
//! пока анимация активна.

use std::hash::Hash;

/// Интерполировать float-значение от текущего к целевому за указанное время.
///
/// Использует `ctx.animate_value_with_time()` внутри.
/// Автоматически вызывает `request_repaint()`, пока анимация активна.
///
/// `duration` — время в секундах (`f32`), например `0.3` для 300ms.
///
/// # Пример
///
/// ```ignore
/// let progress = animate_value(ui, "my_anim", target, 0.3);
/// ```
pub fn animate_value(ui: &egui::Ui, key: impl Hash, target: f32, duration_secs: f32) -> f32 {
    let id = ui.id().with(key);
    ui.ctx().animate_value_with_time(id, target, duration_secs)
}

/// Интерполировать bool → f32 (0.0 → 1.0 или 1.0 → 0.0) за указанное время.
///
/// Использует `ctx.animate_bool_with_time()` внутри.
/// Автоматически вызывает `request_repaint()`, пока анимация активна.
///
/// `duration` — время в секундах (`f32`), например `0.3` для 300ms.
///
/// # Пример
///
/// ```ignore
/// let progress = animate_bool(ui, "visible", is_visible, 0.3);
/// ```
pub fn animate_bool(ui: &egui::Ui, key: impl Hash, target: bool, duration_secs: f32) -> f32 {
    let id = ui.id().with(key);
    ui.ctx().animate_bool_with_time(id, target, duration_secs)
}
