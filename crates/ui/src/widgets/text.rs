//! Виджет [`Text`] — отображает строку текста.
//!
//! Не диспатчит сообщения. Используется как замена `ui.label(...)`.

use egui_android_core::{widget::Widget, Dispatcher};

/// Виджет текста.
pub struct Text {
    text: String,
    /// Размер шрифта. По умолчанию (None) — размер из темы egui.
    font_size: Option<f32>,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            font_size: None,
        }
    }

    /// Установить размер шрифта.
    ///
    /// Пример: `Text::new("Привет").font_size(24.0)`.
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }
}

impl<M> Widget<M> for Text {
    fn render(&self, ui: &mut egui::Ui, _dispatch: &Dispatcher<M>) {
        if let Some(size) = self.font_size {
            ui.label(egui::RichText::new(&self.text).size(size));
        } else {
            ui.label(&self.text);
        }
    }
}
