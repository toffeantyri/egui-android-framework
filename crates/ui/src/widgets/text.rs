//! Виджет [`Text`] — отображает строку текста.
//!
//! Не диспатчит сообщения. Используется как замена `ui.label(...)`.
//!
//! По умолчанию текст **не выделяется** и **не перехватывает скролл**.
//! Чтобы включить выделение:
//!
//! ```ignore
//! Text::new("Выделяемый текст").selectable(true).render(ui, dispatch);
//! ```

use egui_android_core::{widget::Widget, Dispatcher};

/// Виджет текста.
pub struct Text {
    text: String,
    font_size: Option<f32>,
    selectable: bool,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            font_size: None,
            selectable: false,
        }
    }

    /// Установить размер шрифта.
    ///
    /// Пример: `Text::new("Привет").font_size(24.0)`.
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }

    /// Разрешить выделение текста.
    ///
    /// По умолчанию `false` — текст не выделяется, скролл проходит сквозь.
    /// Включите, если нужно копировать текст.
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }
}

impl<M> Widget<M> for Text {
    fn render(&self, ui: &mut egui::Ui, _dispatch: &Dispatcher<M>) {
        if self.selectable {
            // Стандартное egui — можно выделить
            if let Some(size) = self.font_size {
                ui.label(egui::RichText::new(&self.text).size(size));
            } else {
                ui.label(&self.text);
            }
        } else {
            // Рендерим текст как чистый painter (как кнопка рисует свой текст).
            // Никакого Label/Sense — текст не перехватывает скролл.
            let font_id = self
                .font_size
                .map(|s| egui::FontId::proportional(s))
                .unwrap_or_else(|| {
                    ui.style()
                        .text_styles
                        .get(&egui::TextStyle::Body)
                        .cloned()
                        .unwrap_or_else(|| egui::FontId::proportional(16.0))
                });

            let galley =
                ui.painter()
                    .layout_no_wrap(self.text.clone(), font_id, ui.visuals().text_color());
            let text_size = galley.size();

            // Резервируем ровно столько места, сколько занимает текст
            let (rect, _response) = ui.allocate_exact_size(text_size, egui::Sense::hover());

            ui.painter_at(rect)
                .galley(rect.left_top(), galley, ui.visuals().text_color());
        }
    }
}
