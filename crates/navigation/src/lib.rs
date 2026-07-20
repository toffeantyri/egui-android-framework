//! Navigation — древовидная архитектура.
//!
//! Содержит:
//! - [`ChildStack`] — стек дочерних компонентов с управлением lifecycle
//!
//! Зависит от `egui-android-core` (для Component, LifecycleObserver)
//! и `egui-android-ui` (для Modifier, builders).
//! НЕ знает про runtime, platform.

pub mod child_stack;
pub mod child_stack_manager;
pub mod component_factory;
pub mod component_state;

pub use child_stack::ChildStack;
pub use child_stack_manager::ChildStackManager;
pub use component_factory::{create_component, ComponentFactory, ComponentFactory2};
pub use component_state::ComponentState;
