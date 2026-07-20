//! Сохранение/восстановление состояния навигации и компонентов.
//!
//! Аналог `SavedState` + `StateKeeper` из Decompose.
//!
//! # `SavedState`
//!
//! Тип `SavedState` — обёртка над `Option<Vec<u8>>`, представляющая
//! сериализованные данные, которые передаются через Android `Bundle`
//! между `onSaveInstanceState` и `onCreate`.
//!
//! Данные внутри — результат `bincode::serialize(&SavedStack<C>)`,
//! где `SavedStack<C>` содержит конфигурации стека и состояния компонентов.
//!
//! # `SavedStack<C>`
//!
//! Сериализуемое представление стека навигации. Используется:
//! - `ChildStack::save()` → `SavedStack<C>` → `bincode::serialize` → `SavedState`
//! - `SavedState` → `bincode::deserialize` → `SavedStack<C>` → `ChildStack::restore()`
//!
//! # Поток данных
//!
//! ```text
//! Android: onSaveInstanceState(Bundle)
//!   → Application::on_save_state() → SavedState
//!     → bincode::serialize(&SavedStack<C>)
//!       → Bundle.putByteArray("egui_saved_state", bytes)
//!
//! Android: onCreate(Bundle?)
//!   → Bundle.getByteArray("egui_saved_state")
//!     → Application::on_restore_state(SavedState)
//!       → bincode::deserialize::<SavedStack<C>>(&bytes)
//!         → ChildStack::restore_from_saved()
//! ```
//!
//! # Примечание
//!
//! Этот модуль находится в `egui-android-runtime`, а не в navigation,
//! потому что `SavedState` — инфраструктурный тип, используемый
//! на уровне `Application` и `platform-android`, которые не должны
//! знать про `navigation`.

use serde::{Deserialize, Serialize};

/// Тип сохранённого состояния приложения.
///
/// `None` — состояние не сохранялось (первый запуск).
/// `Some(Vec<u8>)` — сериализованные данные для восстановления.
///
/// Содержит `bincode::serialize(&SavedStack<C>)` для конкретного `C`.
/// Тип `C` известен только приложению (например, `Route` в showcase).
pub type SavedState = Option<Vec<u8>>;

/// Сериализуемое представление стека навигации.
///
/// Содержит конфигурации и сериализованные состояния компонентов
/// для каждого элемента стека.
///
/// # Параметры типа
///
/// * `C` — тип конфигурации (маршрут). Должен быть `Serialize + Deserialize`.
///
/// # Пример
///
/// ```ignore
/// let stack: SavedStack<Route> = child_stack.save();
/// let bytes = bincode::serialize(&stack)?;
///
/// // После пересоздания:
/// let stack: SavedStack<Route> = bincode::deserialize(&bytes)?;
/// child_stack.restore_from_saved(stack, &factory);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedStack<C> {
    /// Элементы стека: (конфигурация, сериализованное состояние компонента).
    ///
    /// `None` для компонентов, которые не реализуют `PersistentState`.
    pub items: Vec<(C, Option<Vec<u8>>)>,
}

// Пользовательские Display/Debug уже через derive.
