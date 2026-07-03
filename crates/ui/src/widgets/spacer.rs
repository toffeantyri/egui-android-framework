//! Виджет [`Spacer`] — пустое пространство.
//!
//! Добавляет отступ по ширине, высоте или в обеих осях.
//! Не диспатчит сообщения.

use egui::Sense;
use egui_android_core::{widget::Widget, Dispatcher};

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
    fn render(&self, ui: &mut egui::Ui, _dispatch: &Dispatcher<M>) {
        match (self.width, self.height) {
            (Some(w), Some(h)) => {
                ui.allocate_exact_size(egui::vec2(w, h), Sense::hover());
            }
            (Some(w), None) => {
                ui.add_space(w);
            }
            (None, Some(h)) => {
                ui.add_space(h);
            }
            (None, None) => {
                // Пустой spacer — ничего не делаем
            }
        }
    }
}
