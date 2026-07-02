//! Виджет [`Button`] — кликабельная кнопка.
//!
//! Диспатчит сообщение при клике. Использует builder pattern
//! для задания `on_click`.
//!
//! В отличие от `egui::Button`, резервирует область на всю доступную ширину.
//! Вся область кнопки кликабельна, а текст выравнивается по центру.

use egui_android_core::{widget::Widget, Dispatcher};

/// Виджет кнопки.
///
/// При клике диспатчит сообщение, заданное через [`Button::on_click`].
///
/// Кнопка занимает всю доступную ширину, имеет фиксированную высоту 48.0
/// (задаётся через [`Button::height`]) и выравнивает текст по центру.
/// Вся область кнопки реагирует на клик.
pub struct Button<M> {
    text: String,
    on_click: Option<M>,
    height: f32,
}

impl<M> Button<M> {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            on_click: None,
            height: 48.0,
        }
    }
    pub fn on_click(mut self, msg: M) -> Self {
        self.on_click = Some(msg);
        self
    }
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
            if let Some(msg) = &self.on_click {
                dispatch.dispatch(msg.clone());
            }
        }
    }
}
