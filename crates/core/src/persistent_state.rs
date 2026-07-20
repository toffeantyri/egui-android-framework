//! Трейт [`PersistentState`] для типобезопасного сохранения/восстановления
//! состояния компонента.
//!
//! Аналог `StateKeeper` из Decompose.
//!
//! # Мотивация
//!
//! `ComponentNode::save_state()` возвращает `Option<Box<dyn Any + Send>>`,
//! что несовместимо с сериализацией через `bincode`. `PersistentState`
//! предоставляет типобезопасную альтернативу: компонент объявляет
//! свой тип `State: Serialize + Deserialize`, и blanket-impl в
//! [`super::component_node`] автоматически конвертирует его в `Vec<u8>`.
//!
//! # Пример
//!
//! ```ignore
//! use serde::{Serialize, Deserialize};
//! use egui_android_core::{PersistentState, Component, LifecycleObserver};
//!
//! #[derive(Serialize, Deserialize)]
//! struct MySavedState {
//!     counter: i32,
//! }
//!
//! struct MyScreen {
//!     counter: i32,
//! }
//!
//! impl PersistentState for MyScreen {
//!     type State = MySavedState;
//!
//!     fn save(&self) -> Self::State {
//!         MySavedState { counter: self.counter }
//!     }
//!
//!     fn restore(&mut self, state: Self::State) {
//!         self.counter = state.counter;
//!     }
//! }
//! ```
//!
//! # Рекурсивное сохранение
//!
//! Компонент может сохранять вложенные `ChildStack`:
//!
//! ```ignore
//! impl PersistentState for NestedScreen {
//!     type State = NestedSavedState;  // содержит SavedStack<NestedRoute>
//!     // ...
//! }
//! ```

use serde::{de::DeserializeOwned, Serialize};

/// Трейт для типобезопасного сохранения/восстановления состояния компонента.
///
/// Компонент реализует этот трейт, если хочет сохранять кастомные данные
/// при пересоздании Activity (поворот экрана, kill/restore).
///
/// # Параметры типа
///
/// * `State` — тип сохраняемого состояния.
///   Должен быть `Serialize + DeserializeOwned + Send + 'static`.
pub trait PersistentState {
    /// Тип сохраняемого состояния.
    ///
    /// Должен реализовывать `Serialize` и `DeserializeOwned` для
    /// сериализации через `bincode`.
    type State: Serialize + DeserializeOwned + Send + 'static;

    /// Сохранить текущее состояние компонента.
    ///
    /// Возвращает значение, которое будет сериализовано в `Vec<u8>`
    /// и сохранено в `SavedStack`.
    fn save(&self) -> Self::State;

    /// Восстановить состояние компонента из ранее сохранённого.
    ///
    /// Вызывается после создания компонента (через `ComponentFactory`),
    /// до первого `render()`.
    fn restore(&mut self, state: Self::State);

    /// Хелпер: сериализовать состояние в `Box<dyn Any + Send>`
    /// для использования в `ComponentNode::save_state()`.
    ///
    /// Пример:
    /// ```ignore
    /// fn save_state(&self) -> Option<Box<dyn Any + Send>> {
    ///     PersistentState::save_to_boxed(self)
    /// }
    /// ```
    fn save_to_boxed(&self) -> Option<Box<dyn std::any::Any + Send>> {
        let state = self.save();
        let bytes = bincode::serialize(&state).ok()?;
        Some(Box::new(bytes))
    }

    /// Хелпер: десериализовать состояние из `Box<dyn Any + Send>`.
    ///
    /// Пример:
    /// ```ignore
    /// fn restore_state(&mut self, state: Box<dyn Any + Send>) {
    ///     PersistentState::restore_from_boxed(self, state);
    /// }
    /// ```
    fn restore_from_boxed(&mut self, state: Box<dyn std::any::Any + Send>) {
        if let Ok(bytes) = state.downcast::<Vec<u8>>() {
            if let Ok(saved) = bincode::deserialize::<Self::State>(&bytes) {
                self.restore(saved);
            }
        }
    }
}
