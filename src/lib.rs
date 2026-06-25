pub mod android;
pub mod application;
pub mod child_stack;
pub mod component;
pub mod component_context;
pub mod error;
pub mod lifecycle;
pub mod view;

pub use application::{AppConfig, AppState, Application, DataLayerHandle};
pub use child_stack::ChildStack;
pub use component::{Component, DataEvent};
pub use component_context::{ComponentContext, ComponentContextHandle};
pub use error::AppError;
pub use lifecycle::{LifecycleObserver, LifecycleState};
pub use view::ViewFn;

#[cfg(test)]
mod tests;
