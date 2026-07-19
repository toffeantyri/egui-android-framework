//! Runtime — соединяет Platform и UI.
//!
//! Содержит:
//! - [`Application`] — корень DI, владеет RootComponent
//! - [`Dispatcher`] — отправка сообщений из View
//! - [`StateStore`] — реактивное состояние на tokio::sync::watch
//! - [`UiNotifier`] — связь data layer с Runtime
//! - [`RuntimeContext`] — контекст выполнения для Platform
//! - [`AppError`] — типы ошибок
//!
//! Этот крейт НЕ знает про core, ui, navigation.

pub mod application;
pub mod dispatcher;
pub mod dyn_dispatcher;
pub mod error;
pub mod message_envelope;
pub mod runtime_context;
pub mod store;
pub mod ui_notifier;

pub use application::*;
pub use dispatcher::*;
pub use dyn_dispatcher::*;
pub use error::*;
pub use message_envelope::MessageEnvelope;
pub use runtime_context::RuntimeContext;
pub use store::*;
pub use ui_notifier::*;
