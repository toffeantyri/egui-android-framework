//! Типографика — стили текста (заголовки, body, caption).
//!
//! Аналог Typography из Material Design 3.

/// Набор стилей текста для приложения.
///
/// Включает заголовки трёх уровней, основной текст и подпись.
#[derive(Clone, Debug)]
pub struct Typography {
    /// Самый крупный заголовок (32px).
    pub h1: TextStyle,
    /// Заголовок второго уровня (24px).
    pub h2: TextStyle,
    /// Заголовок третьего уровня (18px).
    pub h3: TextStyle,
    /// Основной текст (14px).
    pub body: TextStyle,
    /// Подпись (12px).
    pub caption: TextStyle,
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            h1: TextStyle {
                font_size: 32.0,
                font_weight: FontWeight::Bold,
                color: egui::Color32::WHITE,
            },
            h2: TextStyle {
                font_size: 24.0,
                font_weight: FontWeight::Bold,
                color: egui::Color32::WHITE,
            },
            h3: TextStyle {
                font_size: 18.0,
                font_weight: FontWeight::Medium,
                color: egui::Color32::WHITE,
            },
            body: TextStyle {
                font_size: 14.0,
                font_weight: FontWeight::Normal,
                color: egui::Color32::WHITE,
            },
            caption: TextStyle {
                font_size: 12.0,
                font_weight: FontWeight::Normal,
                color: egui::Color32::from_gray(180),
            },
        }
    }
}

/// Стиль текста: размер шрифта, насыщенность, цвет.
#[derive(Clone, Debug)]
pub struct TextStyle {
    /// Размер шрифта в логических пикселях.
    pub font_size: f32,
    /// Насыщенность шрифта.
    pub font_weight: FontWeight,
    /// Цвет текста.
    pub color: egui::Color32,
}

/// Насыщенность шрифта.
#[derive(Clone, Copy, Debug)]
pub enum FontWeight {
    /// Light (300).
    Light,
    /// Normal / Regular (400).
    Normal,
    /// Medium (500).
    Medium,
    /// Bold (700).
    Bold,
}
