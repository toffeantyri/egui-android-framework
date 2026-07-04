//! Core — MVI-примитивы.
//!
//! Содержит:
//! - [`Component`] — узел дерева навигации
//! - [`ViewFn`] — сигнатура View-функции
//! - [`Widget<M>`] — базовый трейт виджета
//! - [`LifecycleObserver`] — жизненный цикл компонента
//! - [`ComponentContext`] — контекст компонента
//!
//! Зависит от `egui-android-runtime` (для Dispatcher, StateStore).
//! НЕ знает про ui, navigation.

pub mod component;
pub mod component_context;
pub mod lifecycle;
pub mod view;
pub mod widget;

pub use component::*;
pub use component_context::*;
pub use lifecycle::*;
pub use view::*;
pub use widget::*;

// Реэкспорт ключевых типов из runtime для удобства пользователей.
// UI-крейт (egui-android-ui) использует Dispatcher через core, а не напрямую из runtime.
pub use egui_android_runtime::Dispatcher;
pub use egui_android_runtime::StateStore;
