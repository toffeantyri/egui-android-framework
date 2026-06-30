//! Контейнер [`Column`] — вертикальное расположение дочерних виджетов.
//!
//! Аналог `ui.vertical()` в egui или `Column` в Jetpack Compose.

use crate::{dispatcher::Dispatcher, widgets::Widget};

/// Вертикальный контейнер.
///
/// Располагает дочерние виджеты друг под другом.
/// Поддерживает настраиваемый отступ между детьми через [`Column::spacing`].
///
/// # Пример
///
/// ```ignore
/// Column::new(vec![
///     Box::new(Text::new("Первый")),
///     Box::new(Text::new("Второй")),
/// ])
/// .spacing(8.0)
/// .render(ui, dispatch);
/// ```
pub struct Column<M> {
    children: Vec<Box<dyn Widget<M>>>,
    spacing: Option<f32>,
}

impl<M: 'static> Column<M> {
    /// Создать колонку из списка детей.
    pub fn new(children: Vec<Box<dyn Widget<M>>>) -> Self {
        Self {
            children,
            spacing: None,
        }
    }

    /// Создать пустую колонку.
    ///
    /// Дети добавляются через [`Column::child`].
    pub fn empty() -> Self {
        Self {
            children: Vec::new(),
            spacing: None,
        }
    }

    /// Добавить дочерний виджет.
    ///
    /// Принимает любой тип, реализующий [`Widget<M>`].
    pub fn child(mut self, child: impl Widget<M> + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }

    /// Установить отступ между дочерними виджетами.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = Some(spacing);
        self
    }
}

impl<M: 'static> Widget<M> for Column<M> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        ui.vertical(|ui| {
            if let Some(spacing) = self.spacing {
                ui.spacing_mut().item_spacing = egui::vec2(spacing, spacing);
            }
            for child in &self.children {
                child.render(ui, dispatch);
            }
        });
    }
}
