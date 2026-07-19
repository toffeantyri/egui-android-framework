//! Типобезопасное сохранение и восстановление состояния компонента.
//!
//! # Проблема
//!
//! `ComponentNode::save_state()` и `restore_state()` используют `Box<dyn Any + Send>` —
//! это type-erased API. Ошибка типа обнаруживается только в runtime.
//!
//! # Решение
//!
//! `ComponentState` предоставляет типобезопасный API с ассоциированным типом `State`.
//! Компонент реализует `ComponentState` вместо переопределения `save_state()/restore_state()`:
//!
//! ```ignore
//! impl ComponentState for CounterComponent {
//!     type State = u32;
//!
//!     fn save(&self) -> u32 { self.count }
//!     fn restore(&mut self, state: u32) { self.count = state; }
//! }
//! ```
//!
//! После этого `ChildStack::save()`/`restore()` автоматически используют типизированный API.
//!
//! # Важно
//!
//! - `ComponentState` — это **надстройка** над `ComponentNode`
//! - `ChildStack` внутри использует `ComponentNode::save_state()`/`restore_state()`
//! - Компонент переопределяет эти методы, `ComponentState` просто даёт удобную обёртку
//!
//! # Если компонент НЕ реализует `ComponentState`
//!
//! `save_state()` возвращает `None` — состояние не сохраняется.
//! Это безопасно: при Destroy/InitWindow стек навигации просто не восстановится.

use std::any::Any;
use std::fmt::Debug;

use egui_android_core::ComponentNode;

/// Типобезопасное сохранение и восстановление состояния компонента.
///
/// Ассоциированный тип `State` гарантирует, что `restore()` получит
/// тот же тип, который вернул `save()`.
///
/// # Пример
///
/// ```ignore
/// impl ComponentState for MyScreen {
///     type State = MyState;
///
///     fn save(&self) -> MyState { self.state.clone() }
///     fn restore(&mut self, state: MyState) { self.state = state; }
/// }
/// ```
pub trait ComponentState: ComponentNode {
    /// Тип сохраняемого состояния.
    type State: Clone + Debug + Send + 'static;

    /// Сохранить текущее состояние.
    fn save(&self) -> Self::State;

    /// Восстановить ранее сохранённое состояние.
    fn restore(&mut self, state: Self::State);
}

/// Вспомогательные функции для работы с ComponentState через ComponentNode.
///
/// Позволяют сохранять/восстанавливать состояние компонента, не зная его точный тип.
/// Используются внутри `ChildStack::save()` и `ChildStack::restore()`.
pub mod util {
    use super::*;

    /// Сохранить состояние компонента, если он реализует ComponentState.
    ///
    /// Если компонент НЕ реализует `ComponentState` — возвращает `None`.
    /// Если реализует — возвращает `Some(Box::new(state))`.
    pub fn save_state(node: &dyn ComponentNode) -> Option<Box<dyn Any + Send>> {
        node.save_state()
    }

    /// Восстановить состояние компонента, если он реализует ComponentState.
    ///
    /// Делегирует вызов `ComponentNode::restore_state()`.
    pub fn restore_state(node: &mut dyn ComponentNode, state: Box<dyn Any + Send>) {
        node.restore_state(state);
    }
}
