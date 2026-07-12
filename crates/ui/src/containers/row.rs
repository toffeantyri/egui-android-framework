//! Контейнер [`Row`] — горизонтальное расположение дочерних виджетов.
//!
//! Аналог `Row` в Jetpack Compose. Использует замыкание вместо builder pattern.
//!
//! # Пример
//!
//! ```ignore
//! Row::new(ui, dispatch, |ui, dispatch| {
//!     Text::new("Левый").render(ui, dispatch);
//!     Text::new("Правый").render(ui, dispatch);
//! });
//! ```

use egui_android_core::{Constraints, Dispatcher, UiWrapper};

/// Контейнер с горизонтальным расположением дочерних виджетов.
///
/// По умолчанию spacing между элементами — 8.0.
/// Можно переопределить через [`Row::spacing`].
pub struct Row {
    spacing: f32,
}

impl Default for Row {
    fn default() -> Self {
        Self { spacing: 8.0 }
    }
}

impl Row {
    /// Создать Row с замыканием-рендером и spacing по умолчанию (8.0).
    ///
    /// # Параметры
    ///
    /// * `ui` — текущий Ui
    /// * `dispatch` — диспетчер сообщений
    /// * `content` — замыкание, в котором рендерятся дочерние виджеты
    #[allow(clippy::new_ret_no_self)]
    pub fn new<M: 'static, F>(ui: &mut UiWrapper, dispatch: &Dispatcher<M>, content: F)
    where
        F: FnOnce(&mut UiWrapper, &Dispatcher<M>),
    {
        Self::default().render(ui, dispatch, content);
    }

    /// Установить spacing между элементами.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Рендер с заданным spacing.
    fn render<M: 'static, F>(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<M>, content: F)
    where
        F: FnOnce(&mut UiWrapper, &Dispatcher<M>),
    {
        // Row — горизонтальный контейнер. Ширина детей — wrap-content (0..INF),
        // высота — доступная высота Row (чтобы fill_max_height мог растянуть).
        let available = ui.available_size();
        let child_constraints = Constraints::ranged(0.0, f32::INFINITY, 0.0, available.y.max(0.0));

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(self.spacing, self.spacing);
            content(&mut UiWrapper::new(ui, child_constraints), dispatch);
        });
    }
}
