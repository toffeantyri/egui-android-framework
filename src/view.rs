//! View — чистая функция от состояния, возвращающая сообщения через Dispatcher.
//!
//! В отличие от [`Component`], View не хранит состояние, не знает
//! о каналах и data layer. Это просто функция:
//!
//! ```ignore
//! fn my_view(state: &MyState, ui: &mut egui::Ui, dispatch: &Dispatcher<MyMessage>) {
//!     if ui.button("+").clicked() {
//!         dispatch.dispatch(MyMessage::Increment);
//!     }
//! }
//! ```
//!
//! Сообщения отправляются через [`Dispatcher::dispatch()`] в момент события,
//! а не собираются в `Vec` для постобработки.
//!
//! Такая функция легко тестируется, переиспользуется и не содержит
//! побочных эффектов. Вся логика живёт в Component.

use crate::dispatcher::Dispatcher;

/// Сигнатура View-функции.
///
/// `S` — тип состояния, `M` — тип сообщения.
///
/// В отличие от старой сигнатуры (`fn(&S, &mut Ui) -> Vec<M>`),
/// View больше не возвращает список сообщений. Вместо этого
/// сообщения отправляются через `dispatch` в момент события.
pub type ViewFn<S, M> = fn(state: &S, ui: &mut egui::Ui, dispatch: &Dispatcher<M>);
