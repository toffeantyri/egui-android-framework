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
//! # Выравнивание
//!
//! По умолчанию текст выровнен по левому краю. Чтобы отцентрировать внутри
//! выделенного rect (например, при `fill_max_width`):
//!
//! ```ignore
//! Text::new("Заголовок")
//!     .align(egui::Align::Center)
//!     .modifier(Modifier::new().fill_max_width().padding(16.0).background(c.secondary))
//!     .render(ui, dispatch);
//! ```
//!
//! # Совместимость с модификаторами
//!
//! Text правильно отмеряет собственный размер (wrap-content) и корректно
//! работает с padding, background, size, clickable и другими модификаторами
//! как через Modifier value type, так и напрямую.
//!
//! Внутри использует `ui.allocate_exact_size()` с реальным размером galley,
//! что гарантирует корректный layout в Column, Row, Stack и LazyColumn.
//! Текст по умолчанию рисуется от верхнего левого угла выделенного rect;
//! при заданном `align` смещается по горизонтали.
//! Текст переносится по строкам, если не помещается в доступную ширину.

use egui::Align;
use egui_android_core::{widget::Widget, UiWrapper};
use egui_android_runtime::Dispatcher;

/// Виджет текста.
pub struct Text {
    text: String,
    font_size: Option<f32>,
    selectable: bool,
    text_color: Option<egui::Color32>,
    /// Выравнивание текста внутри выделенного rect.
    /// `None` = левый край (по умолчанию).
    align: Option<Align>,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            font_size: None,
            selectable: false,
            text_color: None,
            align: None,
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

    /// Установить цвет текста (переопределяет цвет темы).
    ///
    /// По умолчанию используется `ui.visuals().text_color()` (on_background из темы).
    /// Задавайте явно, если фон текста отличается от стандартного (например, при
    /// `.background(secondary)` нужно передать `on_secondary` для контраста).
    ///
    /// # Пример
    ///
    /// ```ignore
    /// Text::new("Текст на secondary")
    ///     .text_color(c.on_secondary)
    ///     .modifier(Modifier::new().background(c.secondary))
    ///     .render(ui, dispatch);
    /// ```
    pub fn text_color(mut self, color: egui::Color32) -> Self {
        self.text_color = Some(color);
        self
    }

    /// Выравнивание текста по горизонтали внутри выделенного rect.
    ///
    /// По умолчанию текст прижат к левому краю. Передайте `Align::Center`
    /// для центрирования (полезно с `fill_max_width()`).
    ///
    /// # Пример
    ///
    /// ```ignore
    /// Text::new("Заголовок")
    ///     .align(egui::Align::Center)
    ///     .modifier(Modifier::new().fill_max_width().padding(16.0).background(c.secondary))
    ///     .render(ui, dispatch);
    /// ```
    pub fn align(mut self, align: Align) -> Self {
        self.align = Some(align);
        self
    }
}

impl<M: Send> Widget<M> for Text {
    fn render(&self, ui: &mut UiWrapper, _dispatch: &Dispatcher<M>) {
        if self.selectable {
            if let Some(size) = self.font_size {
                ui.label(egui::RichText::new(&self.text).size(size));
            } else {
                ui.label(&self.text);
            }
        } else {
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
                let max_width = ui.available_width().max(1.0);

                let galley = ui.painter().layout_job(egui::text::LayoutJob {
                    text: self.text.clone(),
                    sections: vec![egui::text::LayoutSection {
                        leading_space: 0.0,
                        byte_range: egui::text::ByteIndex(0)
                            ..egui::text::ByteIndex(self.text.len()),
                        format: egui::text::TextFormat {
                            font_id,
                            color: self.text_color.unwrap_or_else(|| ui.visuals().text_color()),
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

                // Если задан align — alloc'им на доступную ширину, чтобы align имел смысл.
                // Иначе — только под текст (wrap-content).
                let alloc_size = if self.align.is_some() {
                    let avail_w = ui.available_width().max(text_size.x);
                    egui::vec2(avail_w, text_size.y)
                } else {
                    text_size
                };
                let (rect, _response) = ui.allocate_exact_size(alloc_size, egui::Sense::hover());

                // Отладочный лог убран — спамил logcat каждую секунду.
                // При необходимости включить: раскомментировать вызов throttled_log в debug_log.rs.
                let _uid = (rect.min.y * 1000.0) as u64;
                let _ = _uid;

                // Вычисляем позицию текста: по умолчанию левый верхний угол,
                // при align учитываем разницу между шириной rect и текста.
                let mut text_pos = egui::pos2(rect.left(), rect.top());
                if let Some(align) = self.align {
                    let extra_x = rect.width() - text_size.x;
                    if extra_x > 0.0 {
                        text_pos.x = rect.left() + align.to_factor() * extra_x;
                    }
                }

                ui.painter_at(rect).galley(text_pos, galley, text_color);
            } else {
                ui.allocate_space(egui::vec2(0.0, 0.0));
            }
        }
    }
}
