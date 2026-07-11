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
//! как через Modifier value type, так и напрямую.
//!
//! Внутри использует `ui.allocate_space()` с реальным размером galley,
//! что гарантирует корректный layout в Column, Row, Stack и LazyColumn.
//! Текст рисуется от верхнего левого угла выделенного rect (не центрируется
//! по вертикали) — это соответствует контракту top_down layout и делает
//! поведение Text единообразным с Button, Icon и Spacer.
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
                let text_color = galley
                    .job
                    .sections
                    .first()
                    .map(|s| s.format.color)
                    .unwrap_or_else(|| ui.visuals().text_color());

                // alloc'им wrap-content размер через allocate_exact_size.
                let (rect, _response) = ui.allocate_exact_size(text_size, egui::Sense::hover());

                // Рисуем текст от верхнего левого угла rect (без центрирования).
                let text_pos = egui::pos2(rect.left(), rect.top());
                ui.painter_at(rect).galley(text_pos, galley, text_color);
            } else {
                // Пустой текст — alloc'им нулевой размер
                ui.allocate_space(egui::vec2(0.0, 0.0));
            }
        }
    }
}
