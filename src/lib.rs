pub mod android;
pub mod application;
pub mod child_stack;
pub mod component;
pub mod component_context;
pub mod dispatcher;
pub mod error;
pub mod lifecycle;
pub mod modifiers;
pub mod store;
pub mod ui_notifier;
pub mod view;
pub mod widgets;

pub use application::{AppConfig, AppState, Application, DataLayerHandle};
pub use child_stack::ChildStack;
pub use component::Component;
pub use component_context::{ComponentContext, ComponentContextHandle};
pub use dispatcher::Dispatcher;
pub use error::AppError;
pub use lifecycle::{LifecycleObserver, LifecycleState};
pub use modifiers::{Aligned, Background, Clickable, ModifierExt, Padded, SizedWidget};
pub use store::StateStore;
pub use ui_notifier::{AndroidWakeHandle, UiNotifier};
pub use view::ViewFn;
pub use widgets::{Button, Icon, Spacer, Text, Widget};

#[cfg(test)]
mod tests;
