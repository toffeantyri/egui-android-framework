//! Контекст выполнения Runtime — [`RuntimeContext`].
//!
//! Владеет `UiNotifier` и предоставляет платформе единую точку входа.
//!
//! # Архитектура
//!
//! ```text
//! Platform (platform-android)
//!   └── app.create_runtime_context(ctx, waker) → RuntimeContext
//!   └── rt_ctx.check()                           ← единственный вызов
//!         └── UiNotifier::check()
//!               ├── ctx.request_repaint()
//!               └── waker.wake()
//! ```
//!
//! Platform-android видит только `RuntimeContext` и вызывает `check()`.
//! Platform-android НЕ знает про `UiNotifier`, `Waker`, каналы.
//! Вся логика оповещения инкапсулирована внутри Runtime.

use crate::ui_notifier::UiNotifier;

/// Контекст выполнения Runtime.
///
/// Владеет опциональным `UiNotifier` и предоставляет платформе
/// единственный публичный метод — `check()`.
///
/// Создаётся в `Application::create_runtime_context()` и передаётся
/// платформе, которая хранит его и вызывает `check()` в главном цикле.
pub struct RuntimeContext {
    notifier: Option<UiNotifier>,
}

impl RuntimeContext {
    /// Создать новый контекст выполнения.
    ///
    /// `notifier` — опциональный уведомитель. Если `None`, вызов `check()`
    /// ничего не делает (например, для приложений без data layer).
    pub fn new(notifier: Option<UiNotifier>) -> Self {
        Self { notifier }
    }

    /// Проверить наличие сигнала об изменении состояния.
    ///
    /// Единственный публичный метод, который видит платформа.
    /// Вызывается синхронно в главном цикле.
    ///
    /// Если signal есть — вызывает `request_repaint()` и `waker.wake()`.
    pub fn check(&mut self) {
        if let Some(ref mut n) = self.notifier {
            n.check();
        }
    }

    /// Есть ли активный notifier (создан ли data layer).
    pub fn is_active(&self) -> bool {
        self.notifier.is_some()
    }
}
