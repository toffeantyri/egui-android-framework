use std::marker::PhantomData;
use egui_android_core::{widget::Widget, Dispatcher};

pub struct Fade<W, M> {
    pub(crate) inner: W,
    pub(crate) opacity: f32,
    pub(crate) _marker: PhantomData<M>,
}

impl<W, M> Fade<W, M> {
    pub fn new(inner: W, opacity: f32) -> Self {
        Self { inner, opacity: opacity.clamp(0.0, 1.0), _marker: PhantomData }
    }
}

impl<W: Widget<M>, M> Widget<M> for Fade<W, M> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        ui.scope(|ui| {
            ui.multiply_opacity(self.opacity);
            self.inner.render(ui, dispatch);
        });
    }
}
