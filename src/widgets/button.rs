//! Виджет [`Button`] — кликабельная кнопка.
//!
//! Диспатчит сообщение при клике. Использует builder pattern
//! для задания `on_click`.
//!
//! В отличие от `egui::Button`, резервирует область на всю доступную ширину.
//! Вся область кнопки кликабельна, а текст выравнивается по центру.

use crate::{dispatcher::Dispatcher, widgets::Widget};

/// Виджет кнопки.
///
/// При клике диспатчит сообщение, заданное через [`Button::on_click`].
///
/// Кнопка занимает всю доступную ширину, имеет фиксированную высоту 48.0
/// (задаётся через [`Button::height`]) и выравнивает текст по центру.
/// Вся область кнопки реагирует на клик.
///
/// # Пример
///
/// ```ignore
/// Button::new("+1")
///     .on_click(Msg::Increment)
///     .render(ui, dispatch);
/// ```
pub struct Button<M> {
    text: String,
    on_click: Option<M>,
    height: f32,
}

impl<M> Button<M> {
    /// Создать кнопку с текстом.
    ///
    /// Сообщение для диспатча задаётся отдельно через [`Button::on_click`].
    /// Высота по умолчанию — 48.0 логических пикселей.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            on_click: None,
            height: 48.0,
        }
    }

    /// Задать сообщение, которое будет отправлено при клике.
    pub fn on_click(mut self, msg: M) -> Self {
        self.on_click = Some(msg);
        self
    }

    /// Задать высоту кнопки.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }
}

impl<M: Clone> Widget<M> for Button<M> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        let available_width = ui.available_width();
        let desired_size = egui::vec2(available_width, self.height);

        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

        // Заливаем фон кнопки
        if ui.is_rect_visible(rect) {
            let painter = ui.painter_at(rect);
            let rounding = 4.0;
            painter.rect_filled(rect, rounding, egui::Color32::from_rgb(0, 128, 255));
        }

        // Рисуем текст по центру
        let text = egui::RichText::new(&self.text)
            .color(egui::Color32::WHITE)
            .size(18.0);
        let galley = ui.painter().layout_no_wrap(
            text.text().to_owned(),
            egui::FontId::proportional(18.0),
            egui::Color32::WHITE,
        );
        let text_pos = egui::pos2(
            rect.center().x - galley.size().x / 2.0,
            rect.center().y - galley.size().y / 2.0,
        );
        ui.painter_at(rect)
            .galley(text_pos, galley, egui::Color32::WHITE);

        // Обрабатываем клик по всей области кнопки
        if response.clicked() {
            if let Some(msg) = &self.on_click {
                dispatch.dispatch(msg.clone());
            }
        }
    }
}
