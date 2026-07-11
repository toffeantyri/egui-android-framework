//! UI — модификаторы, remember, виджеты и контейнеры.
//!
//! Содержит:
//! - `widgets` — готовые компоненты (Button, Text, Spacer, Icon)
//! - `containers` — контейнеры компоновки (Column, Row, Stack, LazyColumn)
//! - `animation` — анимационные обёртки (Fade, Slide, AnimatedVisibility)
//! - `modifier` — Modifier value type (fill_max_width, padding, background, clickable, ...)
//! - `remember` — UI-only состояние (hover, focus, scroll)

pub mod animation;
pub mod containers;
pub(crate) mod debug_log;
pub mod modifier;
pub mod remember;
pub mod theme;
pub mod widgets;

pub use animation::{
    animate_bool, animate_value, AnimatedVisibility, AnimationExt, Fade, Slide, SlideDirection,
};
pub use containers::{Column, LazyColumn, Row, Stack};
pub use modifier::{Modified, Modifier, ModifierDsl};
pub use remember::*;
pub use theme::{ColorPalette, FontWeight, MaterialTheme, Shapes, TextStyle, Theme, Typography};
pub use widgets::{Button, ButtonColors, Icon, Spacer, Text, Widget};

// Re-export из core
pub use egui_android_core::{Constraints, UiWrapper};
