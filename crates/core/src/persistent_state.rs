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

use crate::{Component, ComponentNode, UiWrapper};
use egui_android_runtime::DynDispatcher;
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
    fn save_to_boxed(&self) -> Option<Box<dyn std::any::Any + Send>> {
        let state = self.save();
        let bytes = bincode::serialize(&state).ok()?;
        Some(Box::new(bytes))
    }

    /// Хелпер: десериализовать состояние из `Box<dyn Any + Send>`.
    fn restore_from_boxed(&mut self, state: Box<dyn std::any::Any + Send>) {
        if let Ok(bytes) = state.downcast::<Vec<u8>>() {
            if let Ok(saved) = bincode::deserialize::<Self::State>(&bytes) {
                self.restore(saved);
            }
        }
    }
}

/// Структурная обёртка, реализующая `ComponentNode` через `Component` + `PersistentState`.
///
/// Используется для компонентов, которые хотят автоматическое
/// сохранение/восстановление состояния через `PersistentState`.
///
/// В отличие от blanket-impl (который вернул бы `save_state = None`),
/// эта обёртка явно реализует `ComponentNode` и делегирует
/// `save_state`/`restore_state` в `PersistentState::save_to_boxed`.
///
/// # Почему не blanket-impl
///
/// Rust запрещает два blanket-impl для одного трейта на пересекающихся типах.
/// Мы не можем сделать:
/// ```ignore
/// impl<T: Component> ComponentNode for T { /* save_state = None */ }
/// impl<T: Component + PersistentState> ComponentNode for T { /* save_state = Some(...) */ } // ❌ конфликт
/// ```
/// `PersistentComponent<T>` — compositional solution, не конфликтующий с blanket-impl.
///
/// # Пример
///
/// ```ignore
/// // StateScreen: Component + PersistentState (через #[derive(Component)])
/// // В фабрике:
/// Box::new(PersistentComponent::new(StateScreen::new()))
/// ```
pub struct PersistentComponent<T> {
    pub inner: T,
}

impl<T> PersistentComponent<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: crate::LifecycleObserver> crate::LifecycleObserver for PersistentComponent<T> {}

impl<T> ComponentNode for PersistentComponent<T>
where
    T: Component + PersistentState,
    T::Message: 'static + Send,
{
    fn render(&self, ui: &mut UiWrapper, dispatch: &DynDispatcher) {
        let typed = dispatch.wrap::<T::Message>();
        Component::render(&self.inner, ui, &typed);
    }

    fn handle_dyn(&mut self, msg: Box<dyn std::any::Any + Send>) {
        if let Ok(typed) = msg.downcast::<T::Message>() {
            Component::handle(&mut self.inner, *typed);
        } else {
            log::error!(
                "PersistentComponent::handle_dyn: ожидался {}, получен неизвестный",
                std::any::type_name::<T::Message>()
            );
        }
    }

    fn handle_back(&mut self) -> bool {
        false
    }

    fn save_state(&self) -> Option<Box<dyn std::any::Any + Send>> {
        PersistentState::save_to_boxed(&self.inner)
    }

    fn restore_state(&mut self, state: Box<dyn std::any::Any + Send>) {
        PersistentState::restore_from_boxed(&mut self.inner, state);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
