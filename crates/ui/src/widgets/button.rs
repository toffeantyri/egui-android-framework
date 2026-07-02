//! Виджет [`Button`] — кликабельная кнопка.
//!
//! Диспатчит сообщение при клике или вызывает closure.
//! Использует builder pattern для задания `on_click` / `on_click_with`.
//!
//! В отличие от `egui::Button`, резервирует область на всю доступную ширину.
//! Вся область кнопки кликабельна, а текст выравнивается по центру.

use egui_android_core::{widget::Widget, Dispatcher};

/// Виджет кнопки.
///
/// При клике:
/// 1. Диспатчит сообщение, заданное через [`Button::on_click`] (если задано).
/// 2. Вызывает closure, заданный через [`Button::on_click_with`] (если задан).
///
/// Оба обработчика могут быть заданы одновременно — сначала диспатчится
/// сообщение, затем вызывается closure.
///
/// Кнопка занимает всю доступную ширину, имеет фиксированную высоту 48.0
/// (задаётся через [`Button::height`]) и выравнивает текст по центру.
/// Вся область кнопки реагирует на клик.
pub struct Button<M> {
    text: String,
    on_click_msg: Option<M>,
    on_click_callback: Option<Box<dyn Fn(&egui::Ui, &Dispatcher<M>)>>,
    height: f32,
}

impl<M: 'static> Button<M> {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            on_click_msg: None,
            on_click_callback: None,
            height: 48.0,
        }
    }

    /// Установить сообщение, которое будет диспатчиться при клике.
    ///
    /// Сообщение проходит через стандартный MVI-поток
    /// (Dispatcher → Component::handle → Data Layer → StateStore).
    pub fn on_click(mut self, msg: M) -> Self {
        self.on_click_msg = Some(msg);
        self
    }

    /// Установить closure, который будет вызван при клике.
    ///
    /// Используется для локальных UI-действий (например, изменение `remember`),
    /// которые не требуют MVI-потока.
    ///
    /// Closure получает `&egui::Ui` и `&Dispatcher<M>` — может читать `remember`,
    /// модифицировать локальное состояние и/или диспатчить сообщения.
    ///
    /// # Пример
    ///
    /// ```ignore
    /// let count = remember(ui, "counter", || 0i32);
    /// Button::new("+1")
    ///     .on_click_with({
    ///         let count = count.clone();
    ///         move |_ui, _dispatch| {
    ///             count.modify(|c| *c += 1);
    ///         }
    ///     })
    ///     .render(ui, dispatch);
    /// ```
    pub fn on_click_with<F>(mut self, callback: F) -> Self
    where
        F: Fn(&egui::Ui, &Dispatcher<M>) + 'static,
    {
        self.on_click_callback = Some(Box::new(callback));
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }
}

impl<M: Clone + 'static> Widget<M> for Button<M> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        let available_width = ui.available_width();
        let desired_size = egui::vec2(available_width, self.height);
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
        if ui.is_rect_visible(rect) {
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(0, 128, 255));
        }
        let galley = ui.painter().layout_no_wrap(
            self.text.clone(),
            egui::FontId::proportional(18.0),
            egui::Color32::WHITE,
        );
        let text_pos = egui::pos2(
            rect.center().x - galley.size().x / 2.0,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter_at(rect)
            .galley(text_pos, galley, egui::Color32::WHITE);
        if response.clicked() {
            // 1. Диспатчим сообщение (MVI-поток)
            if let Some(msg) = &self.on_click_msg {
                dispatch.dispatch(msg.clone());
            }
            // 2. Вызываем closure (локальное UI-действие)
            if let Some(callback) = &self.on_click_callback {
                callback(ui, dispatch);
            }
        }
    }
}
