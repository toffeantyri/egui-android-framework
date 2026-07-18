//! Виджет [`Icon`] — отображает изображение.
//!
//! Использует `egui::Image<'static>` для рендеринга.
//! Не диспатчит сообщения.

use egui_android_core::{widget::Widget, UiWrapper};
use egui_android_runtime::Dispatcher;

/// Виджет-иконка.
pub struct Icon {
    icon: egui::Image<'static>,
}

impl Icon {
    pub fn new(icon: egui::Image<'static>) -> Self {
        Self { icon }
    }
}

impl<M: Send> Widget<M> for Icon {
    fn render(&self, ui: &mut UiWrapper, _dispatch: &Dispatcher<M>) {
        ui.add(self.icon.clone());
    }
}
