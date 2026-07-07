//! Виджет [`Spacer`] — пустое пространство.
//!
//! Добавляет отступ по ширине, высоте или в обеих осях.
//! Не диспатчит сообщения.

use egui::Sense;
use egui_android_core::{widget::Widget, Dispatcher, UiWrapper};

/// Виджет-отступ.
pub struct Spacer {
    width: Option<f32>,
    height: Option<f32>,
}

impl Spacer {
    /// Вертикальный отступ (старый API — совместимость).
    pub fn new(size: f32) -> Self {
        Self {
            width: None,
            height: Some(size),
        }
    }

    /// Универсальный spacer с заданными шириной и высотой.
    pub fn size(width: f32, height: f32) -> Self {
        Self {
            width: Some(width),
            height: Some(height),
        }
    }

    /// Только горизонтальный отступ (для использования внутри Row).
    pub fn width(w: f32) -> Self {
        Self {
            width: Some(w),
            height: None,
        }
    }

    /// Только вертикальный отступ (для использования внутри Column).
    pub fn height(h: f32) -> Self {
        Self {
            width: None,
            height: Some(h),
        }
    }
}

impl<M> Widget<M> for Spacer {
    fn render(&self, ui: &mut UiWrapper, _dispatch: &Dispatcher<M>) {
        // Spacer — только явные размеры, fill_max_width на него не влияет.
        let size = match (self.width, self.height) {
            (Some(w), Some(h)) => egui::vec2(w, h),
            (Some(w), None) => egui::vec2(w, 0.0),
            (None, Some(h)) => egui::vec2(0.0, h),
            (None, None) => return,
        };
        ui.allocate_exact_size(size, Sense::hover());
    }
}
