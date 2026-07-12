//! Контейнер [`Stack`] — наложение дочерних виджетов друг на друга.
//!
//! Аналог `Box` в Jetpack Compose — все дети рендерятся поверх друг друга,
//! начиная с первого (нижний слой) и заканчивая последним (верхний слой).
//!
//! Размер Stack определяется содержимым (максимальный размер детей) — wrap-content.
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

use egui_android_core::{Constraints, Dispatcher, UiWrapper};

/// Контейнер для наложения виджетов друг на друга (аналог `Box` в Compose).
///
/// Размер Stack определяется содержимым (максимальный размер детей) — wrap-content.
///
/// Дети рендерятся один раз. Размер области alloc'ируется после рендера
/// по реальному размеру контента. Если размер контента больше доступного —
/// используется доступный размер (обрезание по родителю).
pub struct Stack;

impl Stack {
    /// Создать Stack с замыканием-рендером.
    ///
    /// # Параметры
    ///
    /// * `ui` — текущий Ui
    /// * `dispatch` — диспетчер сообщений
    /// * `content` — замыкание, в котором рендерятся дочерние виджеты
    #[allow(clippy::new_ret_no_self)]
    pub fn new<M: 'static>(
        ui: &mut UiWrapper,
        dispatch: &Dispatcher<M>,
        content: impl FnOnce(&mut UiWrapper, &Dispatcher<M>),
    ) {
        // Алгоритм: создаём child_ui с max_rect = available_rect_before_wrap,
        // рендерим детей, измеряем min_size, alloc'им в родителе.
        let inner_rect = ui.available_rect_before_wrap();
        let layout = *ui.layout();
        let mut child_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("stack")
                .max_rect(inner_rect)
                .layout(layout),
        );
        // Stack — наложение. Дети могут быть любого размера (wrap-content),
        // но fill_max_width должен работать в пределах доступной ширины.
        let child_constraints = Constraints::ranged(
            0.0,
            inner_rect.width().max(0.0),
            0.0,
            inner_rect.height().max(0.0),
        );
        content(
            &mut UiWrapper::new(&mut child_ui, child_constraints),
            dispatch,
        );
        let content_size = child_ui.min_size();

        // Аллоцируем в родителе ровно по размеру контента (wrap-content)
        let (_rect, _response) = ui.allocate_exact_size(content_size, egui::Sense::hover());

        // Если alloc'нутый размер отличается от inner_rect,
        // контент визуально остаётся в child_ui (который может быть больше).
        // alloc в родителе просто резервирует место для внешнего layout.
    }
}
