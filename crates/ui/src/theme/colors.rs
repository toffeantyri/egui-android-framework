//! Палитра цветов — аналог ColorScheme из Material Design 3.

/// Палитра цветов темы.
#[derive(Clone, Debug)]
pub struct ColorPalette {
    pub primary: egui::Color32,
    pub on_primary: egui::Color32,
    pub secondary: egui::Color32,
    pub on_secondary: egui::Color32,
    pub background: egui::Color32,
    pub on_background: egui::Color32,
    pub surface: egui::Color32,
    pub on_surface: egui::Color32,
    pub error: egui::Color32,
    pub on_error: egui::Color32,
}
