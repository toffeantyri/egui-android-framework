//! Core — MVI-примитивы.
//!
//! Содержит:
//! - [`Component`] — узел дерева навигации
//! - [`ViewFn`] — сигнатура View-функции
//! - [`Widget<M>`] — базовый трейт виджета
//! - [`LifecycleObserver`] — жизненный цикл компонента
//! - [`ComponentContext`] — контекст компонента
//! - [`Constraints`] — Compose-like ограничения min/max width/height
//! - [`UiWrapper`] — обёртка над egui::Ui с поддержкой Constraints
//!
//! Зависит от `egui-android-runtime` (для Dispatcher, StateStore).
//! НЕ знает про ui, navigation.

pub mod back_dispatcher;
pub mod component;
pub mod component_context;
pub mod constraints;
pub mod lifecycle;
pub mod theme;
pub mod ui_wrapper;
pub mod view;
pub mod widget;

pub use back_dispatcher::{BackCallback, BackDispatcher, BackHandling};
pub use component::*;
pub use component_context::*;
pub use constraints::Constraints;
pub use lifecycle::*;
pub use theme::SystemTheme;
pub use ui_wrapper::UiWrapper;
pub use view::*;
pub use widget::*;

// Реэкспорт ключевых типов из runtime для удобства пользователей.
// UI-крейт (egui-android-ui) использует Dispatcher через core, а не напрямую из runtime.
pub use egui_android_runtime::Dispatcher;
pub use egui_android_runtime::StateStore;
