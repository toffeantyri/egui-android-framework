//! Виджет [`Spacer`] — пустое пространство.
//!
//! Добавляет вертикальный отступ. Не диспатчит сообщения.

use egui_android_core::{widget::Widget, Dispatcher};

/// Виджет-отступ.
pub struct Spacer {
    size: f32,
}

impl Spacer {
    pub fn new(size: f32) -> Self {
        Self { size }
    }
}

impl<M> Widget<M> for Spacer {
    fn render(&self, ui: &mut egui::Ui, _dispatch: &Dispatcher<M>) {
        ui.add_space(self.size);
    }
}
