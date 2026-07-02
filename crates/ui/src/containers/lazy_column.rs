use egui_android_core::{widget::Widget, Dispatcher};

pub type ItemBuilder<M, T> = Box<dyn Fn(&T, &Dispatcher<M>) -> Box<dyn Widget<M>>>;

pub struct LazyColumn<M, T> {
    items: Vec<T>,
    item_spacing: f32,
    item_builder: ItemBuilder<M, T>,
}

impl<M: 'static, T: 'static> LazyColumn<M, T> {
    pub fn new<F>(items: Vec<T>, item_builder: F) -> Self
    where
        F: Fn(&T, &Dispatcher<M>) -> Box<dyn Widget<M>> + 'static,
    {
        Self {
            items,
            item_spacing: 4.0,
            item_builder: Box::new(item_builder),
        }
    }
    pub fn item_spacing(mut self, spacing: f32) -> Self {
        self.item_spacing = spacing;
        self
    }
}

impl<M: 'static, T: 'static> Widget<M> for LazyColumn<M, T> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, self.item_spacing);
            for item in &self.items {
                let widget = (self.item_builder)(item, dispatch);
                widget.render(ui, dispatch);
            }
        });
    }
}
