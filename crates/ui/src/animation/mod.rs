mod animate;
mod animated_visibility;
mod fade;
mod slide;

pub use animate::{animate_bool, animate_value};
pub use animated_visibility::AnimatedVisibility;
pub use fade::Fade;
pub use slide::{Slide, SlideDirection};

use egui_android_core::widget::Widget;

pub trait AnimationExt<M>: Widget<M> + Sized {
    fn fade(self, opacity: f32) -> Fade<Self, M>;
    fn slide(self, direction: SlideDirection, offset: f32) -> Slide<Self, M>;
}

impl<T: Widget<M>, M> AnimationExt<M> for T {
    fn fade(self, opacity: f32) -> Fade<Self, M> {
        Fade::new(self, opacity)
    }
    fn slide(self, direction: SlideDirection, offset: f32) -> Slide<Self, M> {
        Slide::new(self, direction, offset)
    }
}
