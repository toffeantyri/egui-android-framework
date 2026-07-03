//! UI — модификаторы, remember, виджеты и контейнеры.
//!
//! Содержит:
//! - `widgets` — готовые компоненты (Button, Text, Spacer, Icon)
//! - `containers` — контейнеры компоновки (Column, Row, Stack, LazyColumn)
//! - `animation` — анимационные обёртки (Fade, Slide, AnimatedVisibility)
//! - `modifier` — новая Modifier value type + старая ModifierExt (совместимость)
//! - `remember` — UI-only состояние (hover, focus, scroll)

pub mod animation;
pub mod containers;
pub mod modifier;
pub mod remember;
pub mod theme;
pub mod widgets;

pub use animation::{
    animate_bool, animate_value, AnimatedVisibility, AnimationExt, Fade, Slide, SlideDirection,
};
pub use containers::{Column, LazyColumn, Row, Stack};
pub use modifier::{legacy::ModifierExt, ModifiedWidget, Modifier, ModifierExt2};
pub use remember::*;
pub use theme::{ColorPalette, FontWeight, MaterialTheme, Shapes, TextStyle, Theme, Typography};
pub use widgets::{Button, Icon, Spacer, Text, Widget};
