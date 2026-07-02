//! Runtime — соединяет Platform и UI.
//!
//! Содержит:
//! - [`Application`] — корень DI, владеет RootComponent
//! - [`Dispatcher`] — отправка сообщений из View
//! - [`StateStore`] — реактивное состояние на tokio::sync::watch
//! - [`UiNotifier`] — связь data layer с Runtime
//! - [`AppError`] — типы ошибок
//!
//! Этот крейт НЕ знает про core, ui, navigation.
