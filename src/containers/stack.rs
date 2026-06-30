//! Контейнер [`Stack`] — наложение дочерних виджетов друг на друга.
//!
//! Аналог ZStack в SwiftUI или `Box` в Jetpack Compose.
//! Все дети рендерятся в одной области, каждый поверх предыдущего.

use crate::{dispatcher::Dispatcher, widgets::Widget};

/// Контейнер-наложение.
///
/// Все дочерние виджеты рендерятся в одной и той же области,
/// накладываясь друг на друга. Полезно для создания
/// наложений, бейджей, подписей поверх изображений.
///
/// # Пример
///
/// ```ignore
/// Stack::new(vec![
///     Box::new(Text::new("Фон")),
///     Box::new(Text::new("Поверх")),
/// ])
/// .render(ui, dispatch);
/// ```
pub struct Stack<M> {
    children: Vec<Box<dyn Widget<M>>>,
}

impl<M: 'static> Stack<M> {
    /// Создать стек из списка детей.
    pub fn new(children: Vec<Box<dyn Widget<M>>>) -> Self {
        Self { children }
    }

    /// Создать пустой стек.
    ///
    /// Дети добавляются через [`Stack::child`].
    pub fn empty() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    /// Добавить дочерний виджет.
    ///
    /// Принимает любой тип, реализующий [`Widget<M>`].
    pub fn child(mut self, child: impl Widget<M> + 'static) -> Self {
        self.children.push(Box::new(child));
        self
    }
}

impl<M: 'static> Widget<M> for Stack<M> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        let available = ui.available_size();
        for (i, child) in self.children.iter().enumerate() {
            let mut child_ui = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt(egui::Id::new(format!("stack_child_{}", i)))
                    .max_rect(ui.max_rect())
                    .layout(*ui.layout()),
            );
            child.render(&mut child_ui, dispatch);
        }
        // Резервируем место по доступному размеру
        ui.allocate_exact_size(available, egui::Sense::hover());
    }
}
