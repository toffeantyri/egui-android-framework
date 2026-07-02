//! `RememberState<T>` — состояние, сохраняемое между кадрами.
//!
//! Хранится в `egui::Context::data()` через ключ `Id`.
//! Автоматически записывает изменения обратно в контекст.

use std::hash::Hash;

use egui::Ui;

/// Состояние, сохраняемое между кадрами.
///
/// Создаётся через функцию [`remember()`].
/// Хранит значение и автоматически сохраняет его в контексте egui.
///
/// # Методы
///
/// - [`get()`](Self::get) — получить текущее значение
/// - [`set()`](Self::set) — установить новое значение
/// - [`modify()`](Self::modify) — изменить значение через замыкание
pub struct RememberState<T> {
    value: T,
    id: egui::Id,
    ctx: egui::Context,
}

impl<T: Clone + Send + Sync + 'static> RememberState<T> {
    /// Получить текущее значение.
    pub fn get(&self) -> &T {
        &self.value
    }

    /// Установить новое значение и сохранить в контексте egui.
    pub fn set(&mut self, new_value: T) {
        self.value = new_value.clone();
        self.persist(&new_value);
    }

    /// Изменить значение через замыкание и сохранить результат.
    ///
    /// ```ignore
    /// state.modify(|v| *v += 1);
    /// ```
    pub fn modify<F: FnOnce(&mut T)>(&mut self, f: F) {
        f(&mut self.value);
        let cloned = self.value.clone();
        self.persist(&cloned);
    }

    fn persist(&self, value: &T) {
        self.ctx.data_mut(|data| {
            data.insert_temp(self.id, value.clone());
        });
    }
}

/// Сохранить состояние между кадрами.
///
/// При первом вызове с данным ключом инициализирует значение через `init`.
/// При последующих вызовах возвращает сохранённое значение.
///
/// Состояние хранится в `egui::Context::data()` и переживает обычные кадры,
/// но сбрасывается при пересоздании контекста (аналог `remember` в Compose).
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
/// let mut expanded = remember(ui, "expanded", || false);
/// if ui.button("Toggle").clicked() {
///     expanded.modify(|v| *v = !*v);
/// }
pub fn remember<T: Clone + Send + Sync + 'static>(
    ui: &Ui,
    key: impl Hash,
    init: impl FnOnce() -> T,
) -> RememberState<T> {
    let id = ui.id().with(key);
    let ctx = ui.ctx().clone();

    let value = ctx.data(|data| data.get_temp::<T>(id)).unwrap_or_else(|| {
        let initial = init();
        ctx.data_mut(|data| {
            data.insert_temp(id, initial.clone());
        });
        initial
    });

    RememberState { value, id, ctx }
}
