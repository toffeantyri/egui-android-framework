//! Виджет [`Text`] — отображает строку текста с переносом.
//!
//! Не диспатчит сообщения. Используется как замена `ui.label(...)`.
//!
//! # Выравнивание
//!
//! По умолчанию текст выровнен по левому краю. `align` влияет на позицию текста
//! внутри выделенного rect (полезно с `fill_max_width()`).
//!
//! # Выделение (touch)
//!
//! Включить Android-подобное выделение длинным нажатием у виджета `Text` можно
//! через `.selectable(true)` (см. `crates/ui/src/text_selection` и
//! `docs/text-selection-refactor.md`).

use egui::Align;
use egui_android_core::{widget::Widget, UiWrapper};
use egui_android_runtime::Dispatcher;

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
    /// Android-подобное выделение текста (заглушка — этап 1; логика — этап 6).
    selectable: bool,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            font_size: None,
            text_color: None,
            align: None,
            id_salt: None,
            selectable: false,
        }
    }

    /// Включить Android-подобное выделение текста (long-press → слово → ручки → тулбар).
    ///
    /// Заглушка на этапе 1: рендер пока как обычный текст; реальная логика выделения
    /// подключается на этапе 6 (см. `docs/text-selection-refactor.md`).
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
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

            if self.selectable {
                // ── Android-подобное выделение (этап 6) ──
                // Единая точка вывода текста: либо galley с фоновым выделением
                // (ПОД глифами), либо обычный рендер — никогда оба.
                use crate::remember::remember;
                use crate::text_selection::android_behavior::LongPressState;
                use crate::text_selection::drag_handles::{
                    dragged_handle, draw_handles_in_area, handle_positions,
                };
                use crate::text_selection::render::paint_galley_with_selection;
                use crate::text_selection::toolbar::{show_toolbar, ToolbarAction};
                use crate::text_selection::SelectionCore;

                let lp = remember(ui, ("sel_lp", widget_id), LongPressState::default);
                let sel = remember(ui, ("sel_core", widget_id), SelectionCore::default);

                let text_rect = egui::Rect::from_min_size(text_pos, galley.size());
                // Небольшой запас для касания по краю строки.
                let hit_rect = text_rect.expand(16.0);

                let (now, any_down, latest) =
                    ui.input(|i| (i.time, i.pointer.any_down(), i.pointer.latest_pos()));

                let mut lp_state = lp.get().clone();
                let in_text = latest.map_or(false, |p| hit_rect.contains(p));
                let start_in_text = lp_state.press_pos.is_none() && any_down && in_text;
                let active_press = lp_state.press_pos.is_some() || start_in_text;
                let down = any_down && active_press;
                let recognized = lp_state.update_raw(now, down, latest);
                if recognized {
                    log::info!(
                        "SEL-PIPE [Text:{:?}] long-press recognized at {:?}",
                        widget_id,
                        latest
                    );
                }
                lp.set(lp_state);

                let mut core = sel.get().clone();

                // Новое нажатие — сняли подавление (следующий реальный клик вне снова сбросит).
                if core.suppress_tap_outside && any_down {
                    core.suppress_tap_outside = false;
                }

                // Долгое нажатие → выделить слово под пальцем.
                if recognized {
                    if let Some(pos) = latest {
                        core.select_word_at(pos, &galley, text_pos, &self.text);
                        log::info!(
                            "SEL-PIPE [Text:{:?}] word selected {:?} range={:?}",
                            widget_id,
                            core.selected_text,
                            core.selection.as_ref().map(|r| r.as_sorted_char_range())
                        );
                    }
                }

                if let Some(range) = core.selection.filter(|r| !r.is_empty()) {
                    // 1. Фон выделения ПОД глифами.
                    paint_galley_with_selection(ui, &galley, text_pos, &range, text_color);

                    // 2. Ручки (Area, Foreground) — рисуются до тулбара.
                    let (sp, ep) = handle_positions(&galley, text_pos, &range);
                    let accent = ui.visuals().selection.stroke.color;
                    let (s_resp, e_resp) =
                        draw_handles_in_area(ui.ctx(), widget_id, sp, ep, accent);

                    // Перетаскивание ручки → расширение/сужение диапазона.
                    if let (Some(sr), Some(er)) = (&s_resp, &e_resp) {
                        if let Some(handle) = dragged_handle(sr, er) {
                            if let Some(pos) = latest {
                                core.drag_handle(handle, pos, &galley, text_pos);
                                log::info!(
                                    "SEL-PIPE [Text:{:?}] drag {:?} pos={:?} -> selected={:?}",
                                    widget_id,
                                    handle,
                                    pos,
                                    core.selected_text
                                );
                            }
                        }
                    }

                    // 3. Тулбар НАД выделением (поверх ручек).
                    if let Some(bbox) = core.selection_rect {
                        if let Some(action) = show_toolbar(
                            ui.ctx(),
                            widget_id.with("sel_tb"),
                            bbox,
                            false, // read-only: без Cut/Paste
                        ) {
                            match action {
                                ToolbarAction::Copy => {
                                    log::info!(
                                        "SEL-PIPE [Text:{:?}] Copy -> {:?}",
                                        widget_id,
                                        core.selected_text
                                    );
                                    ui.copy_text(core.selected_text.clone());
                                    core.reset();
                                }
                                ToolbarAction::SelectAll => {
                                    core.select_all(&galley, text_pos, &self.text);
                                    // Не сбрасывать выделение, пока палец отпускается над попапом.
                                    core.suppress_tap_outside = true;
                                    log::info!(
                                        "SEL-PIPE [Text:{:?}] SelectAll -> {:?}",
                                        widget_id,
                                        core.selected_text
                                    );
                                }
                                // Cut/Paste недоступны для read-only Text.
                                _ => {}
                            }
                        }
                    }
                } else {
                    // Нет активного выделения → обычный рендер (единый путь).
                    ui.painter_at(rect)
                        .galley(text_pos, galley.clone(), text_color);
                }

                // Тап вне выделения → сброс. Выполняется В КОНЦЕ кадра (после обработки
                // клика тулбара), чтобы кадр отпускания над попапом не стёр выделение
                // до того, как «Всё»/копирование успеет сработать.
                // Подавляется после клика по тулбару (SelectAll), пока палец отпускается.
                if !any_down && core.active && !in_text && !core.suppress_tap_outside {
                    log::info!("SEL-PIPE [Text:{:?}] tap outside -> reset", widget_id);
                    core.reset();
                }

                // commit состояния в remember (иначе выделение «не живёт» между кадрами).
                sel.set(core);
            } else {
                // ── Обычный рендер без выделения ──
                ui.painter_at(rect)
                    .galley(text_pos, galley.clone(), text_color);
            }
        } else {
            ui.allocate_space(egui::vec2(0.0, 0.0));
        }
    }
}
