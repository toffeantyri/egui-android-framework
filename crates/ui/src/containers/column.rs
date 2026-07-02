//! Контейнер [`Column`] — вертикальное расположение дочерних виджетов.

use egui_android_core::{widget::Widget, Dispatcher};

pub struct Column<M> {
    children: Vec<Box<dyn Widget<M>>>,
    spacing: Option<f32>,
}

impl<M: 'static> Column<M> {
    pub fn new(children: Vec<Box<dyn Widget<M>>>) -> Self {
        Self {
            children,
            spacing: None,
        }
    }
    pub fn empty() -> Self {
        Self {
            children: Vec::new(),
            spacing: None,
        }
    }
    pub fn child(mut self, child: impl Widget<M> + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = Some(spacing);
        self
    }
}

impl<M: 'static> Widget<M> for Column<M> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        ui.vertical(|ui| {
            if let Some(spacing) = self.spacing {
                ui.spacing_mut().item_spacing = egui::vec2(spacing, spacing);
            }
            for child in &self.children {
                child.render(ui, dispatch);
            }
        });
    }
}
