pub mod activity;
pub mod android;
pub mod app2;
pub mod application;
pub mod child_stack;
pub mod component;
pub mod component_context;
pub mod error;
pub mod lifecycle;
pub mod view;
pub mod view_model;

pub use activity::Activity;
pub use app2::{AppConfig as AppConfig2, AppState, Application2, DataLayerHandle};
pub use application::{AppConfig, AppContext, Application};
pub use child_stack::ChildStack;
pub use component::{Component, DataEvent};
pub use component_context::{ComponentContext, ComponentContextHandle};
pub use error::AppError;
pub use lifecycle::{LifecycleObserver, LifecycleState};
pub use view::ViewFn;
pub use view_model::{ViewModel, ViewModelContext};

#[cfg(test)]
mod tests;
