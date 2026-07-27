//! Платформенная абстракция — минимальный контракт для платформенного уровня.
//!
//! Содержит:
//! - [`Waker`] — пробуждение event loop платформы
//! - [`SystemTheme`] — системная тема (Light/Dark)
//!
//! Этот крейт НЕ знает про runtime, core, ui, navigation.
//! Конкретные реализации находятся в platform-android, platform-desktop и т.д.

pub mod theme;

pub mod waker;

pub use theme::*;
pub use waker::Waker;
