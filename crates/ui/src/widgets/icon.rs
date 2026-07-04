//! Виджет [`Icon`] — отображает изображение.
//!
//! Использует `egui::Image<'static>` для рендеринга.
//! Не диспатчит сообщения.

use egui_android_core::{widget::Widget, Dispatcher};

/// Виджет-иконка.
pub struct Icon {
    icon: egui::Image<'static>,
}

impl Icon {
    pub fn new(icon: egui::Image<'static>) -> Self {
        Self { icon }
    }
}

impl<M> Widget<M> for Icon {
    fn render(&self, ui: &mut egui::Ui, _dispatch: &Dispatcher<M>) {
        ui.add(self.icon.clone());
    }
}
