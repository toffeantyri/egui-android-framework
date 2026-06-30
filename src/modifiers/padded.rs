//! Модификатор [`Padded`] — добавляет отступы вокруг виджета.

use std::marker::PhantomData;

use crate::{dispatcher::Dispatcher, widgets::Widget};

/// Модификатор, добавляющий отступы вокруг виджета.
///
/// Отступы добавляются как сверху, так и снизу через `ui.add_space(...)`.
///
/// # Пример
///
/// ```ignore
/// Text::new("Привет").padding(16.0).render(ui, dispatch);
/// ```
pub struct Padded<W, M> {
    pub(crate) inner: W,
    pub(crate) padding: f32,
    pub(crate) _marker: PhantomData<M>,
}

impl<W: Widget<M>, M> Widget<M> for Padded<W, M> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        ui.add_space(self.padding);
        self.inner.render(ui, dispatch);
        ui.add_space(self.padding);
    }
}
