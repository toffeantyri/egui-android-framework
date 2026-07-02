//! Контейнер [`LazyColumn`] — скроллируемый вертикальный список.
//!
//! Аналог `LazyColumn` в Jetpack Compose.
//! Каждый элемент рендерится с уникальным id на основе индекса,
//! что позволяет корректно использовать `remember` внутри item_builder.
//!
//! # Пример
//!
//! ```ignore
//! LazyColumn::new(items, ui, dispatch, |item, ui, dispatch| {
//!     let mut state = remember(ui, "clicked", || false);
//!     Text::new(&item.title).render(ui, dispatch);
//! });
//! ```

use egui_android_core::Dispatcher;

/// Скроллируемый вертикальный список с ленивым рендерингом элементов.
pub struct LazyColumn;

impl LazyColumn {
    /// Создать LazyColumn с замыканием-билдером для каждого элемента.
    ///
    /// Каждый элемент получает уникальный `ui.push_id(index)`, что позволяет
    /// использовать `remember()` внутри item_builder без коллизий id.
    ///
    /// # Параметры
    ///
    /// * `items` — коллекция элементов для отображения
    /// * `ui` — текущий Ui
    /// * `dispatch` — диспетчер сообщений
    /// * `item_builder` — замыкание, вызываемое для каждого элемента;
    ///   получает ссылку на элемент, ui и dispatch
    pub fn new<M: 'static, T, F>(
        items: Vec<T>,
        ui: &mut egui::Ui,
        dispatch: &Dispatcher<M>,
        item_builder: F,
    ) where
        F: FnMut(&T, &mut egui::Ui, &Dispatcher<M>),
    {
        let spacing = 4.0;
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, spacing);
            let mut builder = item_builder;
            for (index, item) in items.iter().enumerate() {
                ui.push_id(index, |ui| {
                    builder(item, ui, dispatch);
                });
            }
        });
    }
}
