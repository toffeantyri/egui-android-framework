//! Контейнер [`Column`] — вертикальное расположение дочерних виджетов.
//!
//! Аналог `Column` в Jetpack Compose. Использует замыкание вместо builder pattern.
//!
//! # Пример
//!
//! ```ignore
//! Column::new(ui, dispatch, |ui, dispatch| {
//!     Text::new("Заголовок").render(ui, dispatch);
//!     Button::new("Кнопка").on_click(Msg::Clicked).render(ui, dispatch);
//! });
//! ```

use egui_android_core::Dispatcher;

/// Контейнер с вертикальным расположением дочерних виджетов.
///
/// По умолчанию spacing между элементами — 8.0.
/// Можно переопределить через [`Column::spacing`].
pub struct Column {
    spacing: f32,
}

impl Default for Column {
    fn default() -> Self {
        Self { spacing: 8.0 }
    }
}

impl Column {
    /// Создать Column с замыканием-рендером и spacing по умолчанию (8.0).
    ///
    /// # Параметры
    ///
    /// * `ui` — текущий Ui
    /// * `dispatch` — диспетчер сообщений
    /// * `content` — замыкание, в котором рендерятся дочерние виджеты
    pub fn new<M: 'static, F>(ui: &mut egui::Ui, dispatch: &Dispatcher<M>, content: F)
    where
        F: FnOnce(&mut egui::Ui, &Dispatcher<M>),
    {
        Self::default().render(ui, dispatch, content);
    }

    /// Установить spacing между элементами.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Рендер с заданным spacing.
    fn render<M: 'static, F>(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>, content: F)
    where
        F: FnOnce(&mut egui::Ui, &Dispatcher<M>),
    {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(self.spacing, self.spacing);
            content(ui, dispatch);
        });
    }
}
