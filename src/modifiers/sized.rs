//! Модификатор [`SizedWidget`] — устанавливает фиксированный размер виджета.

use std::marker::PhantomData;

use crate::{dispatcher::Dispatcher, widgets::Widget};

/// Модификатор, задающий фиксированный размер для виджета.
///
/// Резервирует область указанного размера через `ui.allocate_exact_size(...)`
/// и рендерит внутренний виджет внутри неё.
///
/// # Пример
///
/// ```ignore
/// Text::new("Привет").size(200.0, 48.0).render(ui, dispatch);
/// ```
pub struct SizedWidget<W, M> {
    pub(crate) inner: W,
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) _marker: PhantomData<M>,
}

impl<W: Widget<M>, M> Widget<M> for SizedWidget<W, M> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        let (rect, _response) =
            ui.allocate_exact_size(egui::vec2(self.width, self.height), egui::Sense::hover());

        // Рендерим внутренний виджет внутри выделенной области
        let mut child_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("sized_widget")
                .max_rect(rect)
                .layout(*ui.layout()),
        );
        self.inner.render(&mut child_ui, dispatch);
    }
}
