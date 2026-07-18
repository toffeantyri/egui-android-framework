//! Waker для пробуждения event loop.
//!
//! Платформенная абстракция — обёртка над замыканием для пробуждения
//! event loop платформы. Не содержит логики, только передачу вызова.
//!
//! # Пример
//!
//! ```
//! use egui_android_platform::waker::Waker;
//! let w = Waker::new(|| {});
//! w.wake();
//! ```

use std::sync::Arc;

/// Handle для пробуждения event loop платформы.
///
/// Создаётся из замыкания, которое вызывает wake-механизм конкретной платформы
/// (например, `AndroidApp::signal()` для Android).
/// В тестах можно использовать заглушку.
pub struct Waker {
    wake_fn: Arc<dyn Fn() + Send + Sync>,
}

impl Clone for Waker {
    fn clone(&self) -> Self {
        Self {
            wake_fn: Arc::clone(&self.wake_fn),
        }
    }
}

impl Waker {
    /// Создать новый waker.
    pub fn new(wake_fn: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            wake_fn: Arc::new(wake_fn),
        }
    }

    /// Пробудить event loop платформы.
    pub fn wake(&self) {
        (self.wake_fn)();
    }
}
