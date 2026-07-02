//! `RememberState<T>` — состояние, сохраняемое между кадрами.
//!
//! Хранится в `egui::Context::data()` через ключ `Id`.
//! Автоматически записывает изменения обратно в контекст.
//!
//! Внутри использует `Arc<RwLock<T>>`, что позволяет:
//! - вызывать `set()` и `modify()` через `&self` (interior mutability)
//! - клонировать `RememberState<T>` с разделением одного состояния
//! - хранить состояние в `IdTypeMap` через `Arc<RwLock<T>>`

use std::hash::Hash;
use std::sync::{Arc, RwLock};

use egui::Ui;

/// Состояние, сохраняемое между кадрами.
///
/// Создаётся через функцию [`remember()`].
/// Хранит значение и автоматически сохраняет его в контексте egui.
///
/// Внутри использует `Arc<RwLock<T>>`. Благодаря этому:
/// - методы `set()` и `modify()` принимают `&self`, а не `&mut self`
/// - `Clone` разделяет одно состояние (через `Arc::clone`)
/// - `RememberState<T>` можно использовать внутри замыканий контейнеров
///
/// # Методы
///
/// - [`get()`](Self::get) — получить текущее значение
/// - [`set()`](Self::set) — установить новое значение
/// - [`modify()`](Self::modify) — изменить значение через замыкание
pub struct RememberState<T> {
    value: Arc<RwLock<T>>,
    id: egui::Id,
    ctx: egui::Context,
}

impl<T: Clone + Send + Sync + 'static> RememberState<T> {
    /// Получить текущее значение.
    ///
    /// Возвращает `RwLockReadGuard<T>` — умный указатель на значение внутри `RwLock`.
    /// Автоматически разыменовывается до `&T`.
    pub fn get(&self) -> std::sync::RwLockReadGuard<'_, T> {
        self.value.read().expect("RememberState: RwLock poisoned")
    }

    /// Установить новое значение и сохранить в контексте egui.
    ///
    /// Принимает `&self` (не требует `&mut self`).
    pub fn set(&self, new_value: T) {
        *self.value.write().expect("RememberState: RwLock poisoned") = new_value.clone();
        self.persist(&new_value);
    }

    /// Изменить значение через замыкание и сохранить результат.
    ///
    /// Принимает `&self` (не требует `&mut self`).
    ///
    /// ```ignore
    /// state.modify(|v| *v += 1);
    /// ```
    pub fn modify<F: FnOnce(&mut T)>(&self, f: F) {
        f(&mut *self.value.write().expect("RememberState: RwLock poisoned"));
        let cloned = self
            .value
            .read()
            .expect("RememberState: RwLock poisoned")
            .clone();
        self.persist(&cloned);
    }

    fn persist(&self, value: &T) {
        self.ctx.data_mut(|data| {
            data.insert_temp(self.id, value.clone());
        });
    }
}

impl<T: Clone> Clone for RememberState<T> {
    fn clone(&self) -> Self {
        Self {
            value: Arc::clone(&self.value),
            id: self.id,
            ctx: self.ctx.clone(),
        }
    }
}

/// Сохранить состояние между кадрами.
///
/// При первом вызове с данным ключом инициализирует значение через `init`.
/// При последующих вызовах возвращает сохранённое значение.
///
/// Состояние хранится в `egui::Context::data()` как `Arc<RwLock<T>>`,
/// что гарантирует разделение одного состояния между всеми клонами
/// `RememberState<T>` и между кадрами.
///
/// # Особенности
///
/// - Возвращает `RememberState<T>`, методы которого принимают `&self`.
/// - Можно использовать внутри замыканий (`Column::new`, `Row::new`).
/// - Можно клонировать и мутировать через любой клон — все изменения
///   видны во всех клонах.
///
/// # Параметры
///
/// * `ui` — текущий `Ui`, из которого берётся контекст
/// * `key` — уникальный ключ для идентификации состояния в рамках виджета
/// * `init` — замыкание, вызываемое один раз для инициализации начального значения
///
/// # Пример
///
/// ```ignore
/// let count = remember(ui, "counter", || 0i32);
/// count.set(42);
/// count.modify(|c| *c += 1);
/// assert_eq!(*count.get(), 43);
///
/// // Внутри замыкания Column:
/// Column::new(ui, dispatch, |ui, dispatch| {
///     let count = remember(ui, "counter", || 0i32);
///     Text::new(format!("{}", count.get())).render(ui, dispatch);
///     Button::new("+1").on_click({
///         let count = count.clone();
///         move |_| count.modify(|c| *c += 1)
///     }).render(ui, dispatch);
/// });
/// ```
pub fn remember<T: Clone + Send + Sync + 'static>(
    ui: &Ui,
    key: impl Hash,
    init: impl FnOnce() -> T,
) -> RememberState<T> {
    let id = ui.id().with(key);
    let ctx = ui.ctx().clone();

    // Храним Arc<RwLock<T>> в IdTypeMap — все клоны разделяют один RwLock
    // IdTypeMap требует Clone + Send + Sync, Arc<RwLock<T>> выполняет все требования
    let arc = ctx
        .data(|data| data.get_temp::<Arc<RwLock<T>>>(id))
        .unwrap_or_else(|| {
            let initial = init();
            let arc = Arc::new(RwLock::new(initial));
            ctx.data_mut(|data| {
                data.insert_temp(id, Arc::clone(&arc));
            });
            arc
        });

    RememberState {
        value: arc,
        id,
        ctx,
    }
}
