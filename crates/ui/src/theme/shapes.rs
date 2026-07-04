//! Скругления углов.

#[derive(Clone, Debug)]
pub struct Shapes {
    pub small: egui::CornerRadius,
    pub medium: egui::CornerRadius,
    pub large: egui::CornerRadius,
}

impl Default for Shapes {
    fn default() -> Self {
        Self {
            small: egui::CornerRadius::same(4),
            medium: egui::CornerRadius::same(8),
            large: egui::CornerRadius::same(16),
        }
    }
}
