//! Контекст компонента — [`ComponentContext`].
//!
//! Предоставляет:
//! - BackDispatcher для регистрации обработчиков Back (диалоги, BottomSheet).
//! - Флаг `finish_requested` для завершения приложения.
//! - Механизм `request_back()` — единый способ запросить навигацию назад.
//!
//! # Обработка Back
//!
//! Back-логика живёт в [`ChildStack::on_back()`]:
//! 1. `active.handle_back()` — компонент перехватывает (NestedScreen, BackCustomScreen)
//! 2. `ChildStack::pop()` — стандартное поведение
//! 3. Стек пуст → `finish_requested = true`
//!
//! `request_back()` — абстрактное намерение «уйти назад». Экран вызывает его
//! из `Component::handle()` (рисованная кнопка «← Назад») или из `handle_back()`
//! (платформенная кнопка). Оба входа ставят один флаг, который хост читает
//! через `take_back_request()` и выполняет `pop` активного экрана.
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
    /// Флаг: активный компонент запросил навигацию назад.
    ///
    /// Устанавливается через [`ComponentContext::request_back`],
    /// читается и сбрасывается через [`ComponentContext::take_back_request`].
    back_requested: bool,
}

impl ComponentContext {
    /// Создать новый контекст.
    pub fn new() -> Self {
        Self {
            back_dispatcher: BackDispatcher::new(),
            finish_requested: false,
            back_requested: false,
        }
    }

    /// Запросить навигацию назад.
    ///
    /// Единый механизм для рисованных и платформенных кнопок Back:
    /// экран вызывает его, когда хочет закрыться. Фреймворк после обработки
    /// сообщения читает запрос через [`ComponentContext::take_back_request`]
    /// и выполняет `pop` активного экрана.
    pub fn request_back(&mut self) {
        self.back_requested = true;
    }

    /// Забрать и сбросить запрос навигации назад.
    ///
    /// Вызывается хостом навигации после `handle_dyn()` / `handle_back()`.
    /// Возвращает `true`, если компонент запросил back (нужно сделать pop).
    pub fn take_back_request(&mut self) -> bool {
        std::mem::replace(&mut self.back_requested, false)
    }
}

impl Default for ComponentContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_back_sets_flag() {
        let mut ctx = ComponentContext::new();
        assert!(!ctx.take_back_request(), "изначально запрос пуст");
        ctx.request_back();
        assert!(
            ctx.take_back_request(),
            "после request_back флаг установлен"
        );
    }

    #[test]
    fn take_back_request_resets_flag() {
        let mut ctx = ComponentContext::new();
        ctx.request_back();
        assert!(ctx.take_back_request(), "первый вызов забирает true");
        assert!(
            !ctx.take_back_request(),
            "после первого вызова флаг сброшен (false)"
        );
    }

    #[test]
    fn multiple_request_back_collapses_to_one() {
        let mut ctx = ComponentContext::new();
        ctx.request_back();
        ctx.request_back();
        assert!(
            ctx.take_back_request(),
            "несколько request_back = один запрос pop"
        );
        assert!(!ctx.take_back_request(), "запрос единичный и был сброшен");
    }
}
