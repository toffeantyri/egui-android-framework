//! Модификатор [`Aligned`] — выравнивает виджет внутри области.

use std::marker::PhantomData;

use crate::{dispatcher::Dispatcher, widgets::Widget};

/// Модификатор, выравнивающий виджет по горизонтали.
///
/// Использует `ui.with_layout(...)` для изменения выравнивания.
///
/// # Пример
///
/// ```ignore
/// Text::new("По центру").align(egui::Align::Center).render(ui, dispatch);
/// ```
pub struct Aligned<W, M> {
    pub(crate) inner: W,
    pub(crate) align: egui::Align,
    pub(crate) _marker: PhantomData<M>,
}

impl<W: Widget<M>, M> Widget<M> for Aligned<W, M> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        ui.with_layout(egui::Layout::top_down(self.align), |ui| {
            self.inner.render(ui, dispatch);
        });
    }
}
