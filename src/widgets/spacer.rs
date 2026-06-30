//! Виджет [`Spacer`] — пустое пространство.
//!
//! Добавляет вертикальный отступ. Не диспатчит сообщения.

use crate::{dispatcher::Dispatcher, widgets::Widget};

/// Виджет-отступ.
///
/// Добавляет вертикальное пространство. Аналог `ui.add_space(...)`.
///
/// # Пример
///
/// ```ignore
/// Spacer::new(16.0).render(ui, dispatch);
/// ```
pub struct Spacer {
    size: f32,
}

impl Spacer {
    /// Создать отступ указанного размера (в логических пикселях).
    pub fn new(size: f32) -> Self {
        Self { size }
    }
}

impl<M> Widget<M> for Spacer {
    fn render(&self, ui: &mut egui::Ui, _dispatch: &Dispatcher<M>) {
        ui.add_space(self.size);
    }
}
