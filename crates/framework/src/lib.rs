//! Umbrella-крейт — re-экспортирует все крейты фреймворка.
//!
//! # Пример
//!
//! ```ignore
//! use egui_android_framework::runtime::Application;
//! use egui_android_framework::core::Component;
//! use egui_android_framework::platform::Platform;
//! ```

pub use egui_android_core as core;
pub use egui_android_navigation as navigation;
pub use egui_android_platform as platform;
pub use egui_android_platform_android as platform_android;
pub use egui_android_runtime as runtime;
pub use egui_android_ui as ui;
