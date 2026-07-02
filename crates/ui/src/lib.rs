//! UI — композиционные билдеры, модификаторы, remember и готовые виджеты.
//!
//! Содержит:
//! - `widgets` — готовые компоненты (Button, Text, Spacer, Icon)
//! - `containers` — контейнеры компоновки (Column, Row, Stack, LazyColumn)
//! - `animation` — анимационные обёртки (Fade, Slide, AnimatedVisibility)
//! - `builders` — базовые композиционные билдеры (column, row, box)
//! - `modifier` — Modifier trait + расширения (padding, width, clickable)
//! - `remember` — UI-only состояние (hover, focus, scroll)

pub mod animation;
pub mod builders;
pub mod containers;
pub mod modifier;
pub mod remember;
pub mod widgets;

pub use animation::{
    animate_bool, animate_value, AnimatedVisibility, AnimationExt, Fade, Slide, SlideDirection,
};
pub use builders::*;
pub use containers::{Column, LazyColumn, Row, Stack};
pub use modifier::*;
pub use remember::*;
pub use widgets::{Button, Icon, Spacer, Text, Widget};
