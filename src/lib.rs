pub mod android;
pub mod application;
pub mod child_stack;
pub mod component;
pub mod component_context;
pub mod egui_subscriber;
pub mod error;
pub mod lifecycle;
pub mod store;
pub mod ui_notifier;
pub mod view;

pub use application::{AppConfig, AppState, Application, DataLayerHandle};
pub use child_stack::ChildStack;
pub use component::{Component, DataEvent};
pub use component_context::{ComponentContext, ComponentContextHandle};
pub use egui_subscriber::AndroidWakeHandle;
pub use error::AppError;
pub use lifecycle::{LifecycleObserver, LifecycleState};
pub use store::StateStore;
pub use ui_notifier::UiNotifier;
pub use view::ViewFn;

#[cfg(test)]
mod tests;
