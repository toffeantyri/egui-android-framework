//! Определение трейта [`Component`] — узла дерева навигации.
//!
//! В отличие от старого MVVM (Activity + ViewModel), Component
//! объединяет состояние и логику одного экрана или поддерева UI.
//! Рендеринг делегируется чистой View-функции (см. [`super::view`]).
//!
//! # Жизненный цикл
//!
//! Компонент реализует [`LifecycleObserver`] и имеет собственный
//! жизненный цикл.
//! Методы `on_create` / `on_start` / `on_resume` / `on_pause` /
//! `on_stop` / `on_destroy` вызываются фреймворком синхронно
//! с Android lifecycle.

use crate::LifecycleObserver;

/// Компонент — узел дерева навигации.
///
/// Владеет состоянием (`State`), обрабатывает сообщения (`Message`),
/// получает события от data layer и делегирует отрисовку View-функции.
pub trait Component: LifecycleObserver + Send + 'static {
    /// Тип состояния, которое читает View-функция.
    type State: 'static;

    /// Тип сообщения, возвращаемого View-функцией.
    type Message: 'static;

    /// Отрисовать UI через View-функцию и вернуть сообщения.
    ///
    /// Вызывается фреймворком один раз за кадр.
    /// Компонент не должен вызывать `egui_ctx.run()` сам —
    /// это делает `run.rs`.
    fn render(&self, ui: &mut egui::Ui) -> Vec<Self::Message>;

    /// Обработать сообщение от View-функции.
    fn handle(&mut self, msg: Self::Message);

    /// Получить ссылку на текущее состояние.
    fn state(&self) -> &Self::State;
}
