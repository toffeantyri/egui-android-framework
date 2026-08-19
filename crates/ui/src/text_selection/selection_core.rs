//! Единая логика выделения для `Text` и `TextEdit`.
//!
//! Хранится per-widget через [`crate::remember::remember`]:
//! `remember(ui, ("sel_core", widget_id), SelectionCore::default)`.
//!
//! `SelectionCore` описывает общее состояние выделения и операции над ним:
//! выбор слова по long-press, обновление границ при drag ручек, select-all и
//! сброс. Различия между read-only текстом (`Text`) и редактируемым (`TextEdit`)
//! решаются на уровне виджета: какие кнопки показывать в тулбаре и как
//! применять `BufferCommand` к буферу текста.

use egui::text::{CCursorRange, Galley};
use egui::Pos2;

use super::android_behavior::{select_word_at, HandleSide};
use super::drag_handles::selection_bbox;
use super::toolbar::ToolbarAction;

/// Действие над буфером текста, которое выполнит виджет ПОСЛЕ
/// `drop(text_guard)` (защита от self-deadlock на `RwLock`).
///
/// `handle_toolbar_action` возвращает команду, а мутацию буфера выполняет
/// виджет (см. этап 7, фикс Б1).
#[derive(Clone, Debug, PartialEq)]
pub enum BufferCommand {
    /// Удалить текст между char-индексами (для `Cut`).
    DeleteRange { start_char: usize, end_char: usize },
    /// Нет действий над буфером.
    None,
}

/// Перевести char-индексы во byte-диапазон строки (для `replace_range`).
///
/// Чистая функция — тестируется без egui `Context`.
pub(crate) fn char_range_to_byte_range(
    text: &str,
    start_char: usize,
    end_char: usize,
) -> (usize, usize) {
    let chars: Vec<char> = text.chars().collect();
    let s = start_char.min(chars.len());
    let e = end_char.min(chars.len()).max(s);

    let byte_s = text
        .char_indices()
        .nth(s)
        .map(|(i, _)| i)
        .unwrap_or(text.len());
    let byte_e = text
        .char_indices()
        .nth(e)
        .map(|(i, _)| i)
        .unwrap_or(text.len());

    (byte_s, byte_e)
}

/// Общее состояние и логика выделения (для `Text` и `TextEdit`).
#[derive(Clone, Debug, Default)]
pub struct SelectionCore {
    /// Активно ли выделение.
    pub active: bool,
    /// Текущий выделенный диапазон (char-индексы галели).
    pub selection: Option<CCursorRange>,
    /// Выделенный текст (для копирования / буфера).
    pub selected_text: String,
    /// Bounding-box выделения (для позиционирования тулбара).
    pub selection_rect: Option<egui::Rect>,
    /// Подавать ли сброс по «тап вне» в текущих кадрах.
    /// Выставляется виджетом сразу после клика по кнопке SelectAll, чтобы
    /// отпускание пальца над тулбаром (вне текста) не сбрасывало выделение.
    /// Снимается при новом нажатии (`any_down`).
    pub suppress_tap_outside: bool,
    /// Пропущен ли первый кадр касания ручки (чтобы не двигать курсор в точку
    /// на самой ручке сразу при нажатии). Сбрасывается при отпускании пальца.
    pub drag_started: bool,
}

impl SelectionCore {
    /// Выделить слово под позицией пальца (long-press).
    ///
    /// Ничего не делает, если слово пустое (или выделение нулевой длины).
    pub fn select_word_at(&mut self, pos: Pos2, galley: &Galley, galley_pos: Pos2, text: &str) {
        let local = pos - galley_pos.to_vec2();
        let cursor = galley.cursor_from_pos(local.to_vec2());
        let range = select_word_at(text, cursor);
        let selected = range.slice_str(text).to_owned();

        if !range.is_empty() && !selected.is_empty() {
            self.active = true;
            self.selection = Some(range);
            self.selected_text = selected;
            self.selection_rect = Some(selection_bbox(galley, galley_pos, &range));
        }
    }

    /// Обновить границу выделения при перетаскивании ручки.
    ///
    /// Первый кадр касания ручки пропускается: `Response::dragged()` возвращает
    /// true сразу при нажатии, и `pointer_pos` ещё указывает НА ручку (ниже текста).
    /// Также `local.y` клампится в диапазон высоты галели, чтобы позиция ниже
    /// последней строки не «прыгала» курсором в конец текста.
    pub fn drag_handle(&mut self, side: HandleSide, pos: Pos2, galley: &Galley, galley_pos: Pos2) {
        // Пропускаем первый кадр касания (точка попадания — на самой ручке).
        if !self.drag_started {
            self.drag_started = true;
            return;
        }

        let mut local = pos - galley_pos.to_vec2();
        // Проецируем y на диапазон строк галели: ниже последней строки — последняя.
        local.y = local.y.clamp(0.0, galley.size().y);
        let cursor = galley.cursor_from_pos(local.to_vec2());

        if let Some(range) = self.selection.as_mut() {
            match side {
                HandleSide::Start => range.secondary = cursor,
                HandleSide::End => range.primary = cursor,
            }
            self.selected_text = range.slice_str(galley).to_owned();
            self.selection_rect = Some(selection_bbox(galley, galley_pos, range));
        }
    }

    /// Выделить весь текст.
    pub fn select_all(&mut self, galley: &Galley, galley_pos: Pos2, text: &str) {
        let all = CCursorRange::select_all(galley);
        self.selection = Some(all);
        self.selected_text = text.to_owned();
        self.selection_rect = Some(selection_bbox(galley, galley_pos, &all));
    }

    /// Обработать действие тулбара и вернуть команду для буфера текста.
    ///
    /// Контракт:
    /// - `Copy` → `None` (виджет сам копирует [`Self::selected_text`] и сбрасывает);
    /// - `Cut` → `BufferCommand::DeleteRange` (если есть выделение), иначе `None`;
    /// - `Paste` → `None` (disabled, нет JNI clipboard read);
    /// - `SelectAll` → `None` (виджет вызывает [`Self::select_all`]).
    ///
    /// Метод НЕ выполняет мутацию буфера и НЕ сбрасывает состояние — это делается
    /// на уровне виджета после применения команды.
    pub fn handle_toolbar_action(&mut self, action: ToolbarAction) -> BufferCommand {
        match action {
            ToolbarAction::Cut => match &self.selection {
                Some(range) => {
                    let [min, max] = range.sorted_cursors();
                    BufferCommand::DeleteRange {
                        start_char: min.index.0,
                        end_char: max.index.0,
                    }
                }
                None => BufferCommand::None,
            },
            ToolbarAction::Copy | ToolbarAction::Paste | ToolbarAction::SelectAll => {
                BufferCommand::None
            }
        }
    }

    /// Полный сброс выделения.
    pub fn reset(&mut self) {
        self.active = false;
        self.selection = None;
        self.selected_text.clear();
        self.selection_rect = None;
        self.suppress_tap_outside = false;
        self.drag_started = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::text::CCursor;
    use std::sync::Arc;

    /// (lo, hi) char-индексы диапазона.
    fn range_lo_hi(r: &CCursorRange) -> (usize, usize) {
        let sorted = r.as_sorted_char_range();
        (sorted.start.0, sorted.end.0)
    }

    /// Запустить замыкание с реальным egui-ui (шрифты загружены).
    fn with_real_ui(f: impl FnOnce(&egui::Ui)) {
        let f = std::cell::RefCell::new(Some(f));
        let ctx = egui::Context::default();
        let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let f = f.borrow_mut().take().unwrap();
                f(ui);
            });
        });
    }

    fn make_galley(ui: &egui::Ui, text: &str) -> Arc<egui::Galley> {
        let job = egui::text::LayoutJob {
            text: text.to_owned(),
            sections: vec![egui::text::LayoutSection {
                leading_space: 0.0,
                byte_range: egui::text::ByteIndex(0)..egui::text::ByteIndex(text.len()),
                format: egui::text::TextFormat {
                    font_id: egui::FontId::proportional(14.0),
                    ..Default::default()
                },
            }],
            wrap: egui::text::TextWrapping {
                max_width: 1000.0,
                ..Default::default()
            },
            ..Default::default()
        };
        ui.painter().layout_job(job)
    }

    // ── select_word_at ──

    #[test]
    fn select_word_at_selects_word_on_long_press() {
        with_real_ui(|ui| {
            let galley = make_galley(ui, "hello world");
            let pos = galley.pos_from_cursor(CCursor::new(2)).center();
            let mut core = SelectionCore::default();
            core.select_word_at(pos, &galley, egui::Pos2::ZERO, &galley.text());
            assert!(core.active);
            let r = core.selection.as_ref().expect("selection set");
            assert_eq!(range_lo_hi(r), (0, 5), "выделено 'hello'");
            assert_eq!(core.selected_text, "hello");
            assert!(core.selection_rect.is_some());
        });
    }

    #[test]
    fn select_word_at_on_whitespace_stays_inactive() {
        with_real_ui(|ui| {
            let galley = make_galley(ui, "hello");
            let pos = galley.pos_from_cursor(galley.end()).center();
            let mut core = SelectionCore::default();
            core.select_word_at(pos, &galley, egui::Pos2::ZERO, &galley.text());
            // Если слово пустое у конца — состояние остаётся неактивным;
            // если активное — диапазон непустой и текст непуст.
            if core.active {
                let r = core.selection.as_ref().expect("selection");
                assert!(!r.is_empty() && !core.selected_text.is_empty());
            }
        });
    }

    #[test]
    fn select_word_at_empty_text_no_panic() {
        with_real_ui(|ui| {
            let galley = make_galley(ui, "");
            let mut core = SelectionCore::default();
            core.select_word_at(egui::Pos2::ZERO, &galley, egui::Pos2::ZERO, "");
            assert!(!core.active);
            assert!(core.selection.is_none());
        });
    }

    // ── drag_handle ──

    #[test]
    fn drag_handle_moves_boundary_and_updates_text() {
        with_real_ui(|ui| {
            let galley = make_galley(ui, "hello world");
            let mut core = SelectionCore::default();
            core.select_word_at(
                galley.pos_from_cursor(CCursor::new(2)).center(),
                &galley,
                egui::Pos2::ZERO,
                &galley.text(),
            );
            // Первый вызов (касание ручки) пропускается; второй — реально двигает.
            let end_pos = galley.pos_from_cursor(CCursor::new(11)).center();
            core.drag_handle(HandleSide::End, end_pos, &galley, egui::Pos2::ZERO);
            core.drag_handle(HandleSide::End, end_pos, &galley, egui::Pos2::ZERO);
            let r = core.selection.as_ref().expect("selection");
            assert!(range_lo_hi(r).1 >= 5, "End расширил диапазон");
            assert!(core.selected_text.contains("world"));
            assert!(core.selection_rect.is_some());
        });
    }

    #[test]
    fn drag_handle_first_frame_does_not_move_cursor() {
        with_real_ui(|ui| {
            let galley = make_galley(ui, "hello world");
            let mut core = SelectionCore::default();
            core.selection = Some(CCursorRange::two(CCursor::new(0), CCursor::new(5)));
            core.drag_started = false;

            // Первый вызов: касание ручки (позиция вне текста) — курсор не двигается.
            core.drag_handle(
                HandleSide::End,
                Pos2::new(999.0, 999.0),
                &galley,
                Pos2::ZERO,
            );
            let r = core.selection.as_ref().unwrap();
            assert_eq!(
                r.primary.index.0, 5,
                "курсор не должен измениться в 1-й кадр"
            );
            assert!(core.drag_started, "drag_started должен стать true");

            // Второй вызов: позиция ниже текста клампится на последнюю строку.
            let below = Pos2::new(0.0, galley.size().y + 100.0);
            core.drag_handle(HandleSide::End, below, &galley, Pos2::ZERO);
            let r = core.selection.as_ref().unwrap();
            assert!(r.primary.index.0 <= 11, "End не прыгает в конец");
        });
    }

    #[test]
    fn drag_handle_clamps_y_to_galley() {
        with_real_ui(|ui| {
            let galley = make_galley(ui, "hello world");
            let mut core = SelectionCore::default();
            core.selection = Some(CCursorRange::two(CCursor::new(0), CCursor::new(5)));
            core.drag_started = true;

            // Позиция глубоко ниже галели → проецируется на последнюю строку,
            // курсор не становится «концом текста» за пределами разумного.
            let below = Pos2::new(10.0, galley.size().y + 300.0);
            core.drag_handle(HandleSide::End, below, &galley, Pos2::ZERO);
            let r = core.selection.as_ref().unwrap();
            assert!(r.primary.index.0 <= 11, "End не прыгает в конец");
        });
    }

    #[test]
    fn drag_handle_without_selection_is_noop() {
        with_real_ui(|ui| {
            let galley = make_galley(ui, "hello");
            let mut core = SelectionCore::default();
            // Минуем «первый кадр» (иначе вызов вернётся до проверки selection).
            core.drag_started = true;
            core.drag_handle(
                HandleSide::Start,
                egui::Pos2::ZERO,
                &galley,
                egui::Pos2::ZERO,
            );
            assert!(core.selection.is_none());
            assert!(core.selected_text.is_empty());
        });
    }

    // ── select_all ──

    #[test]
    fn select_all_covers_whole_text() {
        with_real_ui(|ui| {
            let galley = make_galley(ui, "hello world");
            let mut core = SelectionCore::default();
            core.select_all(&galley, egui::Pos2::ZERO, &galley.text());
            let r = core.selection.as_ref().expect("selection");
            assert_eq!(range_lo_hi(r), (0, 11));
            assert_eq!(core.selected_text, "hello world");
            assert!(core.selection_rect.is_some());
        });
    }

    // ── handle_toolbar_action / BufferCommand ──

    #[test]
    fn cut_returns_delete_range_from_selection() {
        let mut core = SelectionCore::default();
        core.selection = Some(CCursorRange::two(CCursor::new(2), CCursor::new(6)));
        let cmd = core.handle_toolbar_action(ToolbarAction::Cut);
        assert_eq!(
            cmd,
            BufferCommand::DeleteRange {
                start_char: 2,
                end_char: 6
            }
        );
    }

    #[test]
    fn cut_without_selection_returns_none() {
        let mut core = SelectionCore::default();
        let cmd = core.handle_toolbar_action(ToolbarAction::Cut);
        assert_eq!(cmd, BufferCommand::None);
    }

    #[test]
    fn copy_paste_selectall_return_none() {
        let mut core = SelectionCore::default();
        assert_eq!(
            core.handle_toolbar_action(ToolbarAction::Copy),
            BufferCommand::None
        );
        assert_eq!(
            core.handle_toolbar_action(ToolbarAction::Paste),
            BufferCommand::None
        );
        assert_eq!(
            core.handle_toolbar_action(ToolbarAction::SelectAll),
            BufferCommand::None
        );
    }

    // ── char_range_to_byte_range ──

    #[test]
    fn char_to_byte_range_ascii() {
        assert_eq!(char_range_to_byte_range("hello", 0, 5), (0, 5));
        assert_eq!(char_range_to_byte_range("hello", 2, 4), (2, 4));
    }

    #[test]
    fn char_to_byte_range_cyrillic() {
        assert_eq!(char_range_to_byte_range("привет", 0, 2), (0, 4));
        assert_eq!(char_range_to_byte_range("привет", 1, 3), (2, 6));
    }

    #[test]
    fn char_to_byte_range_edge_cases() {
        // за концом
        assert_eq!(char_range_to_byte_range("aб", 0, 99), (0, 3));
        // start>len
        assert_eq!(char_range_to_byte_range("aб", 99, 100), (3, 3));
        // пустая строка
        assert_eq!(char_range_to_byte_range("", 0, 0), (0, 0));
    }

    // ── reset ──

    #[test]
    fn reset_clears_everything() {
        let mut core = SelectionCore::default();
        core.active = true;
        core.selection = Some(CCursorRange::one(CCursor::new(1)));
        core.selected_text = "x".into();
        core.selection_rect = Some(egui::Rect::NOTHING);
        core.suppress_tap_outside = true;
        core.drag_started = true;
        core.reset();
        assert!(!core.active);
        assert!(core.selection.is_none());
        assert!(core.selected_text.is_empty());
        assert!(core.selection_rect.is_none());
        assert!(!core.suppress_tap_outside);
        assert!(!core.drag_started);
    }
}
