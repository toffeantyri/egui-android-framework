//! UI — модификаторы, remember, виджеты и контейнеры.
//!
//! Содержит:
//! - `widgets` — готовые компоненты (Button, Text, Spacer, Icon)
//! - `containers` — контейнеры компоновки (Column, Row, Stack, LazyColumn)
//! - `animation` — анимационные обёртки (Fade, Slide, AnimatedVisibility)
//! - `modifier` — новая Modifier value type + старая ModifierExt (совместимость)
//! - `remember` — UI-only состояние (hover, focus, scroll)
//! - `constraints` — Compose-like Constraints (min/max width/height)
//! - `ui_wrapper` — обёртка над egui::Ui с поддержкой Constraints

pub mod animation;
pub mod constraints;
pub mod containers;
pub mod modifier;
pub mod remember;
pub mod theme;
pub mod ui_wrapper;
pub mod widgets;

pub use animation::{
    animate_bool, animate_value, AnimatedVisibility, AnimationExt, Fade, Slide, SlideDirection,
};
pub use constraints::Constraints;
pub use containers::{Column, LazyColumn, Row, Stack};
pub use modifier::{legacy::ModifierExt, Modified, Modifier, ModifierApply};
pub use remember::*;
pub use theme::{ColorPalette, FontWeight, MaterialTheme, Shapes, TextStyle, Theme, Typography};
pub use ui_wrapper::UiWrapper;
pub use widgets::{Button, ButtonColors, Icon, Spacer, Text, Widget};
