//! Контейнер [`LazyColumn`] — вертикальный список со скроллингом.
//!
//! Аналог `ScrollArea` в egui или `LazyColumn` в Jetpack Compose.
//! Рендерит все элементы в прокручиваемой области.
//! Виртуализация будет добавлена позже.

use crate::{dispatcher::Dispatcher, widgets::Widget};

/// Тип билдера элементов для [`LazyColumn`].
///
/// Принимает ссылку на элемент данных и Dispatcher,
/// возвращает виджет, реализующий [`Widget<M>`].
pub type ItemBuilder<M, T> = Box<dyn Fn(&T, &Dispatcher<M>) -> Box<dyn Widget<M>>>;

/// Вертикальный список со скроллингом.
///
/// Принимает список данных и функцию-билдер, которая создаёт
/// виджет для каждого элемента. Все элементы рендерятся
/// в `egui::ScrollArea::vertical()`.
///
/// # Пример
///
/// ```ignore
/// let items = vec!["A", "B", "C"];
/// LazyColumn::new(items, |item, dispatch| {
///     Box::new(Text::new(format!("Item {}", item)))
/// })
/// .item_spacing(8.0)
/// .render(ui, dispatch);
/// ```
pub struct LazyColumn<M, T> {
    items: Vec<T>,
    item_spacing: f32,
    item_builder: ItemBuilder<M, T>,
}

impl<M: 'static, T: 'static> LazyColumn<M, T> {
    /// Создать список из элементов и функции-билдера.
    ///
    /// `items` — данные для отображения.
    /// `item_builder` — замыкание, которое для каждого элемента
    /// создаёт виджет.
    pub fn new<F>(items: Vec<T>, item_builder: F) -> Self
    where
        F: Fn(&T, &Dispatcher<M>) -> Box<dyn Widget<M>> + 'static,
    {
        Self {
            items,
            item_spacing: 4.0,
            item_builder: Box::new(item_builder),
        }
    }

    /// Установить отступ между элементами списка.
    pub fn item_spacing(mut self, spacing: f32) -> Self {
        self.item_spacing = spacing;
        self
    }
}

impl<M: 'static, T: 'static> Widget<M> for LazyColumn<M, T> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, self.item_spacing);
            for item in &self.items {
                let widget = (self.item_builder)(item, dispatch);
                widget.render(ui, dispatch);
            }
        });
    }
}
