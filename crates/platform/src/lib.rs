//! Платформенная абстракция — контракт для различных платформ (Android, iOS, Desktop, Web).
//!
//! Определяет трейт [`Platform`], типы [`PlatformEvent`], [`FrameInput`], [`FrameOutput`]
//! и [`PlatformConfig`].
//!
//! Этот крейт НЕ знает про runtime, core, ui, navigation.

pub mod config;
pub mod event;
pub mod frame;
pub mod platform;

pub use config::*;
pub use event::*;
pub use frame::*;
pub use platform::*;
