//! Локальное состояние виджетов — аналог `remember {}` в Jetpack Compose.
//!
//! Позволяет виджетам хранить собственное состояние между кадрами
//! (развёрнут ли аккордеон, значение текстового поля, выбранная вкладка).
//!
//! Не заменяет `StateStore` для глобального бизнес-состояния,
//! а дополняет его для UI-мелочей.
//!
//! # Пример
//!
//! ```ignore
//! use egui_android_framework::remember;
//!
//! let mut expanded = remember(ui, "card_expanded", || false);
//! if ui.button("Toggle").clicked() {
//!     expanded.modify(|v| *v = !*v);
//! }
//! if *expanded.get() {
//!     ui.label("Контент");
//! }
//! ```

mod remember;

pub use remember::{remember, RememberState};
