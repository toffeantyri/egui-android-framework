//! Core — MVI-примитивы.
//!
//! Содержит:
//! - [`Component`] — узел дерева навигации
//! - [`ViewFn`] — сигнатура View-функции
//! - [`LifecycleObserver`] — жизненный цикл компонента
//! - [`ComponentContext`] — контекст компонента
//!
//! Зависит от `egui-android-runtime` (для Dispatcher, StateStore).
//! НЕ знает про ui, navigation.

pub mod component;
pub mod component_context;
pub mod lifecycle;
pub mod view;

pub use component::*;
pub use component_context::*;
pub use lifecycle::*;
pub use view::*;
