//! Контейнер [`Column`] — вертикальное расположение дочерних виджетов.
//!
//! Аналог `Column` в Jetpack Compose. Использует замыкание вместо builder pattern.
//!
//! Поддерживает опциональный скролл через [`Column::scrollable`].
//!
//! # Пример
//!
//! ```ignore
//! // Обычная Column
//! Column::new()
//!     .show(ui, dispatch, |ui, dispatch| {
//!         Text::new("Заголовок").render(ui, dispatch);
//!     });
//!
//! // Скроллируемая Column
//! Column::new()
//!     .scrollable()
//!     .show(ui, dispatch, |ui, dispatch| {
//!         // ... много элементов ...
//!     });
//! ```

use egui_android_core::{Constraints, Dispatcher, UiWrapper};

/// Контейнер с вертикальным расположением дочерних виджетов.
///
/// По умолчанию spacing между элементами — 8.0.
/// Можно переопределить через [`Column::spacing`].
/// Скролл включается через [`Column::scrollable`].
pub struct Column {
    spacing: f32,
    scrollable: bool,
}

impl Default for Column {
    fn default() -> Self {
        Self {
            spacing: 8.0,
            scrollable: false,
        }
    }
}

impl Column {
    /// Создать Column с настройками по умолчанию.
    ///
    /// Для отображения вызови [`Column::show`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Компактный вариант: создать и сразу отобразить Column с содержимым.
    ///
    /// Эквивалентно `Column::new().show(ui, dispatch, content)`.
    /// Если нужен скролл, используй `Column::new().scrollable().show(...)`.
    #[deprecated(
        since = "0.4.0",
        note = "используйте Column::new().show(ui, dispatch, content) или Column::new().scrollable().show(...)"
    )]
    pub fn with_content<M: 'static, F>(ui: &mut UiWrapper, dispatch: &Dispatcher<M>, content: F)
    where
        F: FnOnce(&mut UiWrapper, &Dispatcher<M>),
    {
        Self::default().show(ui, dispatch, content);
    }

    /// Установить spacing между элементами.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Включить вертикальный скролл для Column.
    ///
    /// Если контент не помещается по высоте, Column становится скроллируемой.
    pub fn scrollable(mut self) -> Self {
        self.scrollable = true;
        self
    }

    /// Отобразить Column с указанным содержимым.
    ///
    /// # Параметры
    ///
    /// * `ui` — текущий Ui
    /// * `dispatch` — диспетчер сообщений
    /// * `content` — замыкание, в котором рендерятся дочерние виджеты
    pub fn show<M: 'static, F>(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<M>, content: F)
    where
        F: FnOnce(&mut UiWrapper, &Dispatcher<M>),
    {
        // Передаём дочерним виджетам constraints: ширина = доступная ширина,
        // высота — без ограничений (0..INF). Это позволяет fill_max_width
        // работать корректно: виджет видит max_width = available.x.
        let available = ui.available_size();
        let child_constraints = Constraints::ranged(0.0, available.x.max(0.0), 0.0, f32::INFINITY);

        let render_inner = |ui: &mut egui::Ui| {
            ui.spacing_mut().item_spacing = egui::vec2(self.spacing, self.spacing);
            content(&mut UiWrapper::new(ui, child_constraints), dispatch);
        };

        if self.scrollable {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |scroll_ui| {
                    render_inner(&mut UiWrapper::new(scroll_ui, child_constraints))
                });
        } else {
            ui.vertical(|v_ui| render_inner(&mut UiWrapper::new(v_ui, child_constraints)));
        }
    }
}
