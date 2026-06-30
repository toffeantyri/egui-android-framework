//! Базовые виджеты-структуры для декларативного UI.
//!
//! Вдохновлены Jetpack Compose: виджеты принимают `&Dispatcher<M>`
//! для отправки сообщений в момент событий, а не возвращают `Vec<Message>`.
//!
//! Все виджеты реализуют трейт [`Widget<M>`] — фундамент для модификаторов
//! (Этап 3).
//!
//! # Использование
//!
//! ```ignore
//! use egui_android_framework::widgets::{Text, Button, Spacer, Widget};
//!
//! fn my_view(ui: &mut egui::Ui, dispatch: &Dispatcher<Msg>) {
//!     Text::new("Заголовок").render(ui, dispatch);
//!     Spacer::new(8.0).render(ui, dispatch);
//!     Button::new("+1")
//!         .on_click(Msg::Increment)
//!         .render(ui, dispatch);
//! }
//! ```

mod button;
mod icon;
mod spacer;
mod text;

pub use button::Button;
pub use icon::Icon;
pub use spacer::Spacer;
pub use text::Text;

use crate::dispatcher::Dispatcher;

/// Базовый трейт для всех виджетов.
///
/// Виджет знает только про `egui::Ui` и [`Dispatcher`].
/// Виджет **НЕ** знает про Store, Component, Reducer.
///
/// # Generic
///
/// * `M` — тип сообщения, которое виджет может диспатчить.
pub trait Widget<M> {
    /// Рендерит виджет в UI.
    ///
    /// Может диспатчить сообщения через `dispatch` в момент событий
    /// (клик, переключение, изменение текста и т.д.).
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>);
}
