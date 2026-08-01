//! Core — MVI-примитивы.
//!
//! Содержит:
//! - [`Component`] — узел дерева навигации
//! - [`Widget<M>`] — базовый трейт виджета
//! - [`LifecycleObserver`] — жизненный цикл компонента
//! - [`ComponentContext`] — контекст компонента
//! - [`Constraints`] — Compose-like ограничения min/max width/height
//! - [`UiWrapper`] — обёртка над egui::Ui с поддержкой Constraints
//!
//! Зависит от `egui-android-runtime` (для Dispatcher, StateStore).
//! НЕ знает про ui, navigation.

pub mod back_action;
pub mod back_dispatcher;
pub mod component;
pub mod component_context;
pub mod component_node;
pub mod constraints;
pub mod lifecycle;
pub mod persistent_state;
pub mod ui_wrapper;
pub mod widget;

pub use back_action::BackAction;
pub use back_dispatcher::{BackCallback, BackDispatcher, BackHandling};
pub use component::*;
pub use component_context::*;
pub use component_node::*;
pub use constraints::Constraints;
pub use lifecycle::*;
pub use persistent_state::PersistentState;
pub use ui_wrapper::UiWrapper;
pub use widget::*;
