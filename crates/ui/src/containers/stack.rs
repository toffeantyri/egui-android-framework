use egui_android_core::{widget::Widget, Dispatcher};

pub struct Stack<M> {
    children: Vec<Box<dyn Widget<M>>>,
}

impl<M: 'static> Stack<M> {
    pub fn new(children: Vec<Box<dyn Widget<M>>>) -> Self {
        Self { children }
    }
    pub fn empty() -> Self {
        Self { children: Vec::new() }
    }
    pub fn child(mut self, child: impl Widget<M> + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }
}

impl<M: 'static> Widget<M> for Stack<M> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        let available = ui.available_size();
        for (i, child) in self.children.iter().enumerate() {
            let mut child_ui = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt(egui::Id::new(format!("stack_child_{}", i)))
                    .max_rect(ui.max_rect())
                    .layout(*ui.layout()),
            );
            child.render(&mut child_ui, dispatch);
        }
        ui.allocate_exact_size(available, egui::Sense::hover());
    }
}
