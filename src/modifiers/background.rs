//! Модификатор [`Background`] — устанавливает цвет фона для виджета.

use std::marker::PhantomData;

use crate::{dispatcher::Dispatcher, widgets::Widget};

/// Модификатор, задающий цвет фона для виджета.
///
/// Использует `egui::Frame::NONE.fill(color).show(...)` для отрисовки фона.
///
/// # Пример
///
/// ```ignore
/// Text::new("Привет").background(egui::Color32::DARK_BLUE).render(ui, dispatch);
/// ```
pub struct Background<W, M> {
    pub(crate) inner: W,
    pub(crate) color: egui::Color32,
    pub(crate) _marker: PhantomData<M>,
}

impl<W: Widget<M>, M> Widget<M> for Background<W, M> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        let frame = egui::Frame::NONE.fill(self.color);
        frame.show(ui, |ui| {
            self.inner.render(ui, dispatch);
        });
    }
}
