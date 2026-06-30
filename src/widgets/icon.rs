//! Виджет [`Icon`] — отображает изображение.
//!
//! Использует `egui::Image<'static>` для рендеринга.
//! Не диспатчит сообщения.

use crate::{dispatcher::Dispatcher, widgets::Widget};

/// Виджет-иконка.
///
/// Отображает статическое изображение. Не диспатчит сообщения.
///
/// # Пример
///
/// ```ignore
/// let image = egui::Image::new(egui::include_image!("icon.png"));
/// Icon::new(image).render(ui, dispatch);
/// ```
pub struct Icon {
    icon: egui::Image<'static>,
}

impl Icon {
    /// Создать иконку из egui-изображения.
    pub fn new(icon: egui::Image<'static>) -> Self {
        Self { icon }
    }
}

impl<M> Widget<M> for Icon {
    fn render(&self, ui: &mut egui::Ui, _dispatch: &Dispatcher<M>) {
        ui.add(self.icon.clone());
    }
}
