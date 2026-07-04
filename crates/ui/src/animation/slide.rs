use egui_android_core::{widget::Widget, Dispatcher};
use std::marker::PhantomData;

#[derive(Clone, Copy, Debug)]
pub enum SlideDirection {
    Left,
    Right,
    Up,
    Down,
}

pub struct Slide<W, M> {
    pub(crate) inner: W,
    pub(crate) direction: SlideDirection,
    pub(crate) offset: f32,
    pub(crate) _marker: PhantomData<M>,
}

impl<W, M> Slide<W, M> {
    pub fn new(inner: W, direction: SlideDirection, offset: f32) -> Self {
        Self {
            inner,
            direction,
            offset,
            _marker: PhantomData,
        }
    }
}

impl<W: Widget<M>, M> Widget<M> for Slide<W, M> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        let offset_vec = match self.direction {
            SlideDirection::Left => egui::vec2(-self.offset, 0.0),
            SlideDirection::Right => egui::vec2(self.offset, 0.0),
            SlideDirection::Up => egui::vec2(0.0, -self.offset),
            SlideDirection::Down => egui::vec2(0.0, self.offset),
        };
        ui.scope(|ui| {
            let max_rect = ui.max_rect().translate(offset_vec);
            let mut child_ui = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt("slide")
                    .max_rect(max_rect)
                    .layout(*ui.layout()),
            );
            self.inner.render(&mut child_ui, dispatch);
        });
    }
}
