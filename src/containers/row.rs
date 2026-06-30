//! Контейнер [`Row`] — горизонтальное расположение дочерних виджетов.
//!
//! Аналог `ui.horizontal()` в egui или `Row` в Jetpack Compose.

use crate::{dispatcher::Dispatcher, widgets::Widget};

/// Горизонтальный контейнер.
///
/// Располагает дочерние виджеты в одну строку.
/// Поддерживает настраиваемый отступ между детьми через [`Row::spacing`].
///
/// # Пример
///
/// ```ignore
/// Row::new(vec![
///     Box::new(Text::new("A")),
///     Box::new(Text::new("B")),
/// ])
/// .spacing(16.0)
/// .render(ui, dispatch);
/// ```
pub struct Row<M> {
    children: Vec<Box<dyn Widget<M>>>,
    spacing: Option<f32>,
}

impl<M: 'static> Row<M> {
    /// Создать строку из списка детей.
    pub fn new(children: Vec<Box<dyn Widget<M>>>) -> Self {
        Self {
            children,
            spacing: None,
        }
    }

    /// Создать пустую строку.
    ///
    /// Дети добавляются через [`Row::child`].
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

impl<M: 'static> Widget<M> for Row<M> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        ui.horizontal(|ui| {
            if let Some(spacing) = self.spacing {
                ui.spacing_mut().item_spacing = egui::vec2(spacing, spacing);
            }
            for child in &self.children {
                child.render(ui, dispatch);
            }
        });
    }
}
