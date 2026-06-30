//! Предопределённые темы Material Design 3 — светлая и тёмная.
//!
//! Цвета основаны на Material Design 3 color scheme.

use super::{ColorPalette, Shapes, Theme, Typography};

/// Фабрика предопределённых тем Material Design.
pub struct MaterialTheme;

impl MaterialTheme {
    /// Светлая тема Material Design 3.
    ///
    /// Используется по умолчанию, если тема не установлена.
    pub fn light() -> Theme {
        Theme {
            colors: ColorPalette {
                primary: egui::Color32::from_rgb(62, 100, 255),
                on_primary: egui::Color32::WHITE,
                secondary: egui::Color32::from_rgb(187, 134, 252),
                on_secondary: egui::Color32::WHITE,
                background: egui::Color32::WHITE,
                on_background: egui::Color32::from_rgb(0, 0, 0),
                surface: egui::Color32::from_rgb(245, 245, 245),
                on_surface: egui::Color32::from_rgb(0, 0, 0),
                error: egui::Color32::from_rgb(244, 67, 54),
                on_error: egui::Color32::WHITE,
            },
            typography: Typography::default(),
            shapes: Shapes {
                small: 4.0,
                medium: 8.0,
                large: 16.0,
            },
        }
    }

    /// Тёмная тема Material Design 3.
    pub fn dark() -> Theme {
        Theme {
            colors: ColorPalette {
                primary: egui::Color32::from_rgb(138, 180, 248),
                on_primary: egui::Color32::from_rgb(0, 0, 0),
                secondary: egui::Color32::from_rgb(203, 166, 247),
                on_secondary: egui::Color32::from_rgb(0, 0, 0),
                background: egui::Color32::from_rgb(18, 18, 18),
                on_background: egui::Color32::from_rgb(255, 255, 255),
                surface: egui::Color32::from_rgb(30, 30, 30),
                on_surface: egui::Color32::from_rgb(255, 255, 255),
                error: egui::Color32::from_rgb(244, 67, 54),
                on_error: egui::Color32::WHITE,
            },
            typography: Typography::default(),
            shapes: Shapes {
                small: 4.0,
                medium: 8.0,
                large: 16.0,
            },
        }
    }
}
