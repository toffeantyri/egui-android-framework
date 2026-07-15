//! Базовый трейт [`Widget<M>`] — фундамент для всех виджетов и модификаторов.
//!
//! Виджет знает только про `UiWrapper` и [`Dispatcher`].
//! Виджет **НЕ** знает про Store, Component, Reducer.

use crate::UiWrapper;
use egui_android_runtime::Dispatcher;

/// Базовый трейт для всех виджетов.
///
/// Виджет знает только про `UiWrapper` и [`Dispatcher`].
/// Виджет **НЕ** знает про Store, Component, Reducer.
///
/// # Generic
///
/// * `M` — тип сообщения, которое виджет может диспатчить.
pub trait Widget<M: Send> {
    /// Рендерит виджет в UI.
    ///
    /// Может диспатчить сообщения через `dispatch` в момент событий
    /// (клик, переключение, изменение текста и т.д.).
    fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<M>);
}
