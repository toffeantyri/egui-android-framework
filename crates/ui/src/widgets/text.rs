//! Виджет [`Text`] — отображает строку текста с переносом.
//!
//! Не диспатчит сообщения. Используется как замена `ui.label(...)`.
//!
//! Всегда отрисовывается как обычный виджет фреймворка (единый путь рендера) и
//! публикует свою «текстовую поверхность» ([`crate::text_selection::TextSurface`]),
//! которую может потребить `Modifier::selectable()`:
//!
//! ```ignore
//! Text::new("Выделяемый текст")
//!     .modifier(Modifier::new().selectable(true))
//!     .render(ui, dispatch);
//! ```
//!
//! # Выравнивание
//!
//! По умолчанию текст выровнен по левому краю. `align` влияет на позицию текста
//! внутри выделенного rect (полезно с `fill_max_width()`).
//!
//! # Выделение (touch)
//!
//! Само выделение — это **модификатор** `Modifier::selectable(true)` (см.
//! `crates/ui/src/text_selection`). Он работает с любым виджетом, опубликовавшим
//! `TextSurface`, поэтому переиспользуем.

use egui::Align;
use egui_android_core::{widget::Widget, UiWrapper};
use egui_android_runtime::Dispatcher;

use crate::text_selection::{publish_text_surface, TextSurface};

/// Виджет текста.
pub struct Text {
    text: String,
    font_size: Option<f32>,
    text_color: Option<egui::Color32>,
    /// Выравнивание текста внутри выделенного rect.
    /// `None` = левый край (по умолчанию).
    align: Option<Align>,
    /// Стабильный id виджета (для `Modifier::selectable`). `None` — авто (`ui.next_auto_id()`).
    id_salt: Option<egui::Id>,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            font_size: None,
            text_color: None,
            align: None,
            id_salt: None,
        }
    }

    /// Установить размер шрифта.
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }

    /// Установить цвет текста (переопределяет цвет темы).
    pub fn text_color(mut self, color: egui::Color32) -> Self {
        self.text_color = Some(color);
        self
    }

    /// Выравнивание текста по горизонтали внутри выделенного rect.
    ///
    /// По умолчанию текст прижат к левому краю. Передайте `Align::Center`
    /// для центрирования (полезно с `fill_max_width()`).
    pub fn align(mut self, align: Align) -> Self {
        self.align = Some(align);
        self
    }

    /// Задать явный стабильный id виджета.
    ///
    /// Нужен при нескольких `selectable(true)` рядом, чтобы состояния выделения
    /// не пересекались. По умолчанию id берётся автоматически (`ui.next_auto_id()`).
    pub fn id(mut self, id: egui::Id) -> Self {
        self.id_salt = Some(id);
        self
    }
}

impl<M: Send> Widget<M> for Text {
    fn render(&self, ui: &mut UiWrapper, _dispatch: &Dispatcher<M>) {
        let widget_id = self.id_salt.unwrap_or_else(|| ui.next_auto_id());
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
                    byte_range: egui::text::ByteIndex(0)..egui::text::ByteIndex(self.text.len()),
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

            // Вычисляем позицию текста: по умолчанию левый верхний угол,
            // при align учитываем разницу между шириной rect и текста.
            let mut text_pos = egui::pos2(rect.left(), rect.top());
            if let Some(align) = self.align {
                let extra_x = rect.width() - text_size.x;
                if extra_x > 0.0 {
                    text_pos.x = rect.left() + align.to_factor() * extra_x;
                }
            }

            ui.painter_at(rect)
                .galley(text_pos, galley.clone(), text_color);

            // Публикуем текстовую поверхность для `Modifier::selectable`.
            publish_text_surface(
                ui,
                TextSurface {
                    id: widget_id,
                    galley: galley.clone(),
                    galley_pos: text_pos,
                    text: self.text.clone(),
                },
            );
        } else {
            ui.allocate_space(egui::vec2(0.0, 0.0));
        }
    }
}
