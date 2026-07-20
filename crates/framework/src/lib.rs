//! Umbrella-крейт — re-экспортирует все крейты фреймворка.
//!
//! # Пример
//!
//! ```ignore
//! use egui_android_framework::runtime::Application;
//! use egui_android_framework::core::Component;
//! use egui_android_framework::Component;  // derive-макрос
//! ```

pub extern crate egui_android_core;

pub use egui_android_core as core;
pub use egui_android_navigation as navigation;
pub use egui_android_platform as platform;
pub use egui_android_platform_android as platform_android;
pub use egui_android_runtime as runtime;
pub use egui_android_ui as ui;

// Реэкспорт макросов
pub use egui_android_macros::component;
pub use egui_android_macros::Component;

// Удобный prelude
pub mod prelude {
    pub use egui_android_core::{
        Component, ComponentNode, LifecycleObserver, PersistentState, UiWrapper,
    };
    pub use egui_android_macros::component;
    pub use egui_android_macros::Component as ComponentDerive;
    pub use egui_android_navigation::{ChildStack, ComponentFactory};
    pub use egui_android_platform::Waker;
    pub use egui_android_runtime::{
        Application, Dispatcher, DynDispatcher, SavedStack, SavedState, StateStore,
    };
}
