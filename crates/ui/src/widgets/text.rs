//! Виджет [`Text`] — отображает строку текста с переносом.
//!
//! Не диспатчит сообщения. Используется как замена `ui.label(...)`.
//!
//! По умолчанию текст **не выделяется** и **не перехватывает скролл**.
//! Чтобы включить выделение:
//!
//! ```ignore
//! Text::new("Выделяемый текст").selectable(true).render(ui, dispatch);
//! ```
//!
//! # Совместимость с модификаторами
//!
//! Text правильно отмеряет собственный размер (wrap-content) и корректно
//! работает с padding, background, size, clickable и другими модификаторами
//! как из legacy ModifierExt, так и из новой Modifier value type.
//!
//! Внутри использует `ui.allocate_exact_size()` с реальным размером galley,
//! что гарантирует корректный layout в Column, Row, Stack и LazyColumn.
//! Текст переносится по строкам, если не помещается в доступную ширину.

use egui_android_core::{widget::Widget, Dispatcher, UiWrapper};

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
    fn render(&self, ui: &mut UiWrapper, _dispatch: &Dispatcher<M>) {
        if self.selectable {
            // Стандартное egui — можно выделить
            if let Some(size) = self.font_size {
                ui.label(egui::RichText::new(&self.text).size(size));
            } else {
                ui.label(&self.text);
            }
        } else {
            // Не-выделяемый текст: используем allocate_exact_size с реальным размером galley.
            // Это гарантирует правильную работу с модификаторами (padding, background и т.д.)
            // и корректный layout внутри Column/Row.
            let font_id = self
                .font_size
                .map(egui::FontId::proportional)
                .unwrap_or_else(|| {
                    ui.style()
                        .text_styles
                        .get(&egui::TextStyle::Body)
                        .cloned()
                        .unwrap_or_else(|| egui::FontId::proportional(16.0))
                });

            if !self.text.is_empty() {
                // Определяем максимальную ширину для переноса текста.
                let max_width = ui.available_width().max(1.0);

                let galley = ui.painter().layout_job(egui::text::LayoutJob {
                    text: self.text.clone(),
                    sections: vec![egui::text::LayoutSection {
                        leading_space: 0.0,
                        byte_range: egui::text::ByteIndex(0)
                            ..egui::text::ByteIndex(self.text.len()),
                        format: egui::text::TextFormat {
                            font_id: font_id,
                            color: ui.visuals().text_color(),
                            ..Default::default()
                        },
                    }],
                    wrap: egui::text::TextWrapping {
                        max_width,
                        max_rows: usize::MAX,
                        break_anywhere: true,
                        overflow_character: Some('\u{FFFD}'),
                    },
                    ..Default::default()
                });
                let text_size = galley.size();

                // Если available_width больше wrap-content — alloc'им всю ширину
                // (поддержка fill_max_width через set_min_width родителя).
                let avail_w = ui.available_width();
                let final_width = if avail_w > text_size.x {
                    avail_w
                } else {
                    text_size.x
                };
                let final_size = egui::vec2(final_width, text_size.y);
                let (rect, _response) = ui.allocate_exact_size(final_size, egui::Sense::hover());

                ui.painter_at(rect)
                    .galley(rect.left_top(), galley, ui.visuals().text_color());
            } else {
                // Пустой текст — alloc'им нулевой размер
                ui.allocate_exact_size(egui::vec2(0.0, 0.0), egui::Sense::hover());
            }
        }
    }
}
