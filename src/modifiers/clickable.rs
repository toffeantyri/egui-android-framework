//! Модификатор [`Clickable`] — делает виджет кликабельным.
//!
//! При клике диспатчит указанное сообщение через `Dispatcher`.

use std::marker::PhantomData;

use crate::{dispatcher::Dispatcher, widgets::Widget};

/// Модификатор, делающий виджет кликабельным.
///
/// При клике по области виджета диспатчит заданное сообщение.
///
/// # Пример
///
/// ```ignore
/// Text::new("Нажми меня")
///     .clickable(Msg::Clicked)
///     .render(ui, dispatch);
/// ```
pub struct Clickable<W, M> {
    pub(crate) inner: W,
    pub(crate) msg: M,
    pub(crate) _marker: PhantomData<M>,
}

impl<W: Widget<M>, M: Clone> Widget<M> for Clickable<W, M> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        // Сначала рендерим внутренний виджет, чтобы зарезервировать место
        let (_rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::click());

        let mut child_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("clickable")
                .max_rect(_rect)
                .layout(*ui.layout()),
        );
        self.inner.render(&mut child_ui, dispatch);

        // Проверяем клик по области
        if response.clicked() {
            dispatch.dispatch(self.msg.clone());
        }
    }
}
