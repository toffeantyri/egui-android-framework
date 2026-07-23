//! Контекст компонента — [`ComponentContext`].
//!
//! Предоставляет:
//! - BackDispatcher для регистрации обработчиков Back (диалоги, BottomSheet).
//! - Флаг `finish_requested` для завершения приложения.
//!
//! # Обработка Back
//!
//! Back-логика живёт в [`ChildStack::on_back()`]:
//! 1. `active.handle_back()` — компонент перехватывает (NestedScreen, BackCustomScreen)
//! 2. `ChildStack::pop()` — стандартное поведение
//! 3. Стек пуст → `finish_requested = true`
//!
//! `BackDispatcher` — для будущих сценариев (диалоги, BottomSheet).
//! Не используется в текущей версии.

use crate::back_dispatcher::BackDispatcher;

/// Контекст компонента.
///
/// Не generic. Содержит только инфраструктурные поля.
#[derive(Debug)]
pub struct ComponentContext {
    /// BackDispatcher для регистрации обработчиков Back.
    pub back_dispatcher: BackDispatcher,
    /// Флаг: запрошено завершение приложения.
    ///
    /// Устанавливается, когда `ChildStack` пуст и Back нажат.
    /// Читается `Application::request_destroy()`.
    pub finish_requested: bool,
}

impl ComponentContext {
    /// Создать новый контекст.
    pub fn new() -> Self {
        Self {
            back_dispatcher: BackDispatcher::new(),
            finish_requested: false,
        }
    }
}

impl Default for ComponentContext {
    fn default() -> Self {
        Self::new()
    }
}
