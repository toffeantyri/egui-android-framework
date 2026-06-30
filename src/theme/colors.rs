//! Палитра цветов — аналог ColorScheme из Material Design 3.
//!
//! Содержит основные цвета для UI-компонентов:
//! primary, secondary, surface, background, error и их on-цвета.

/// Палитра цветов темы.
///
/// Следует соглашению Material Design:
/// - `primary` / `on_primary` — основные цвета (кнопки, активные элементы)
/// - `secondary` / `on_secondary` — акцентные цвета
/// - `surface` / `on_surface` — фон карточек и поверхностей
/// - `background` / `on_background` — основной фон
/// - `error` / `on_error` — цвета ошибок
#[derive(Clone, Debug)]
pub struct ColorPalette {
    /// Основной цвет темы.
    pub primary: egui::Color32,
    /// Цвет текста/иконок поверх primary.
    pub on_primary: egui::Color32,
    /// Акцентный цвет.
    pub secondary: egui::Color32,
    /// Цвет текста/иконок поверх secondary.
    pub on_secondary: egui::Color32,
    /// Цвет фона приложения.
    pub background: egui::Color32,
    /// Цвет текста/иконок поверх background.
    pub on_background: egui::Color32,
    /// Цвет поверхностей (карточки, меню).
    pub surface: egui::Color32,
    /// Цвет текста/иконок поверх surface.
    pub on_surface: egui::Color32,
    /// Цвет ошибок.
    pub error: egui::Color32,
    /// Цвет текста/иконок поверх error.
    pub on_error: egui::Color32,
}
