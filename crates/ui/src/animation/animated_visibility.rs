use egui_android_core::{widget::Widget, Dispatcher, UiWrapper};

pub struct AnimatedVisibility<M> {
    visible: bool,
    duration: f32,
    child: Option<Box<dyn Widget<M>>>,
}

impl<M: 'static> AnimatedVisibility<M> {
    pub fn new(visible: bool, duration_secs: f32) -> Self {
        Self {
            visible,
            duration: duration_secs,
            child: None,
        }
    }
    pub fn child(mut self, child: impl Widget<M> + 'static) -> Self {
        self.child = Some(Box::new(child));
        self
    }
}

impl<M: 'static> Widget<M> for AnimatedVisibility<M> {
    fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<M>) {
        let ctx = ui.ctx();
        let id = ui.id().with("animated_visibility");
        let progress = ctx.animate_bool_with_time(id, self.visible, self.duration);
        if progress <= 0.0 {
            return;
        }
        if let Some(child) = &self.child {
            ui.scope(|scope_ui| {
                scope_ui.multiply_opacity(progress);
                let mut w = UiWrapper::new_unconstrained(scope_ui);
                child.render(&mut w, dispatch);
            });
        }
    }
}
