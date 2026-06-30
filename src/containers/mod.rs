//! Контейнеры для компоновки виджетов.
//!
//! Аналог Column, Row, Stack, LazyColumn из Jetpack Compose.
//! Все контейнеры реализуют [`Widget<M>`], что даёт им доступ
//! ко всем модификаторам через [`ModifierExt`].
//!
//! # Использование
//!
//! ```ignore
//! use egui_android_framework::{
//!     containers::{Column, Row, LazyColumn},
//!     widgets::{Text, Button, Widget},
//!     modifiers::ModifierExt,
//! };
//!
//! Column::empty()
//!     .child(Text::new("Заголовок"))
//!     .child(Button::new("Кнопка").on_click(Msg::Clicked))
//!     .spacing(8.0)
//!     .padding(16.0)
//!     .render(ui, dispatch);
//! ```

mod column;
mod lazy_column;
mod row;
mod stack;

pub use column::Column;
pub use lazy_column::LazyColumn;
pub use row::Row;
pub use stack::Stack;
