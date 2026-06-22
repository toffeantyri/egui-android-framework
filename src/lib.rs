pub mod activity;
pub mod android;
pub mod application;
pub mod error;
pub mod lifecycle;
pub mod view_model;

pub use activity::Activity;
pub use application::{AppConfig, AppContext, Application};
pub use error::AppError;
pub use lifecycle::{LifecycleObserver, LifecycleState};
pub use view_model::{ViewModel, ViewModelContext};

#[cfg(test)]
mod tests {
    mod error_tests;
    mod lifecycle_tests;
    mod view_model_tests;
}
