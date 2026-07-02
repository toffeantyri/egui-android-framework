//! Виджет [`Text`] — отображает строку текста.
//!
//! Не диспатчит сообщения. Используется как замена `ui.label(...)`.

use egui_android_core::{widget::Widget, Dispatcher};

/// Виджет текста.
pub struct Text {
    text: String,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl<M> Widget<M> for Text {
    fn render(&self, ui: &mut egui::Ui, _dispatch: &Dispatcher<M>) {
        ui.label(&self.text);
    }
}
