use egui_android_core::{widget::Widget, Dispatcher, UiWrapper};
use std::marker::PhantomData;

pub struct Fade<W, M> {
    pub(crate) inner: W,
    pub(crate) opacity: f32,
    pub(crate) _marker: PhantomData<M>,
}

impl<W, M> Fade<W, M> {
    pub fn new(inner: W, opacity: f32) -> Self {
        Self {
            inner,
            opacity: opacity.clamp(0.0, 1.0),
            _marker: PhantomData,
        }
    }
}

impl<W: Widget<M>, M: Send> Widget<M> for Fade<W, M> {
    fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<M>) {
        ui.scope(|scope_ui| {
            scope_ui.multiply_opacity(self.opacity);
            let mut w = UiWrapper::new_unconstrained(scope_ui);
            self.inner.render(&mut w, dispatch);
        });
    }
}
