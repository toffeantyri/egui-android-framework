//! Виджет [`Text`] — отображает строку текста.
//!
//! Не диспатчит сообщения. Используется как замена `ui.label(...)`.

use crate::{dispatcher::Dispatcher, widgets::Widget};

/// Виджет текста.
///
/// Отображает статический текст. Не принимает сообщения.
///
/// # Пример
///
/// ```ignore
/// Text::new("Привет, мир!").render(ui, dispatch);
/// ```
pub struct Text {
    text: String,
}

impl Text {
    /// Создать текст.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl<M> Widget<M> for Text {
    fn render(&self, ui: &mut egui::Ui, _dispatch: &Dispatcher<M>) {
        ui.label(&self.text);
    }
}
