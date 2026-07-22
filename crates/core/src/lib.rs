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
pub mod component_context2;
pub mod component_node;
pub mod constraints;
pub mod lifecycle;
pub mod persistent_state;
pub mod state_keeper;
pub mod ui_wrapper;
pub mod view;
pub mod widget;

pub use back_dispatcher::{BackCallback, BackDispatcher, BackHandling};
pub use component::*;
pub use component_context::*;
pub use component_context2::*;
pub use component_node::*;
pub use constraints::Constraints;
pub use lifecycle::*;
pub use persistent_state::{PersistentComponent, PersistentState};
pub use state_keeper::{StateKeeper, StateKeeperSnapshot};
pub use ui_wrapper::UiWrapper;
pub use view::*;
pub use widget::*;
