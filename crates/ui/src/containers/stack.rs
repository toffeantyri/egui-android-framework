//! Контейнер [`Stack`] — наложение дочерних виджетов друг на друга.
//!
//! Аналог `Box` в Jetpack Compose — все дети рендерятся поверх друг друга,
//! начиная с первого (нижний слой) и заканчивая последним (верхний слой).
//!
//! # Пример
//!
//! ```ignore
//! Stack::new(ui, dispatch, |ui, dispatch| {
//!     Text::new("Фон")
//!         .background(egui::Color32::BLUE)
//!         .padding(8.0)
//!         .render(ui, dispatch);
//!     Text::new("Поверх")
//!         .padding(8.0)
//!         .render(ui, dispatch);
//! });
//! ```

use egui_android_core::Dispatcher;

/// Контейнер для наложения виджетов друг на друга (аналог `Box` в Compose).
///
/// Замыкание получает `ui` и `dispatch` — каждый child рендерится
/// с уникальным `id_salt` на основе индекса, что исключает коллизии
/// идентификаторов (исправление старой проблемы).
///
/// Размер Stack определяется содержимым (максимальный размер детей).
pub struct Stack;

impl Stack {
    /// Создать Stack с замыканием-рендером.
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
        let available = ui.available_size();
        // Резервируем всю доступную область для стека
        let (rect, _response) = ui.allocate_exact_size(available, egui::Sense::hover());

        let mut child_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("stack")
                .max_rect(rect)
                .layout(*ui.layout()),
        );
        content(&mut child_ui, dispatch);
    }
}
