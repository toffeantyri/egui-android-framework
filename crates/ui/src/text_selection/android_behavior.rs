//! Распознавание длинного нажатия и выделение слова по word-boundary.
//!
//! Чистая логика (без egui-рендера): тестируется на хосте.

use egui::text::{CCursor, CCursorRange, CharIndex};
use egui::{Pos2, Response, Ui};
use unicode_segmentation::UnicodeSegmentation;

/// Порог долгого нажатия (мс).
const LONG_PRESS_MS: f64 = 400.0;
/// Максимальное смещение пальца, чтобы считать удержание «нажатием» (точек).
const MAX_PRESS_DRIFT: f32 = 12.0;

/// Какую ручку выделения тянем.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandleSide {
    Start,
    End,
}

/// Состояние распознавания долгого нажатия для одного виджета.
#[derive(Clone, Debug, Default)]
pub struct LongPressState {
    /// Позиция, где палец опустился.
    pub press_pos: Option<Pos2>,
    /// Время нажатия (секунды egui-времени).
    pub press_time: f64,
    /// Долгое нажатие распознано → режим выделения активен.
    pub selection_active: bool,
    /// Какую ручку сейчас тянем.
    pub active_handle: Option<HandleSide>,
}

impl LongPressState {
    /// Обновить состояние по Response виджета.
    ///
    /// Возвращает `true`, если в этом кадре распознано долгое нажатие
    /// (начинаем выделение слова). Тонкая обёртка над [`Self::update_raw`].
    pub fn update(&mut self, ui: &Ui, response: &Response) -> bool {
        let now = ui.input(|i| i.time);
        let pos = response.interact_pointer_pos();
        let down = response.is_pointer_button_down_on();
        self.update_raw(now, down, pos)
    }

    /// Чистая логика распознавания долгого нажатия (без `Ui`/`Response` — тестируема).
    ///
    /// `now` — время (сек, egui-время); `is_down_on` — палец удержан на виджете;
    /// `pointer_pos` — позиция пальца. Возвращает `true`, если распознано.
    pub fn update_raw(&mut self, now: f64, is_down_on: bool, pointer_pos: Option<Pos2>) -> bool {
        // Палец опущен на виджете и режим выделения ещё не активен.
        if is_down_on && !self.selection_active {
            if self.press_pos.is_none() {
                self.press_pos = pointer_pos;
                self.press_time = now;
            }
            let held_ms = (now - self.press_time) * 1000.0;
            let drifted = pointer_pos
                .zip(self.press_pos)
                .map_or(false, |(a, b)| a.distance(b) > MAX_PRESS_DRIFT);

            if held_ms >= LONG_PRESS_MS && !drifted {
                self.selection_active = true;
                self.press_pos = None;
                return true;
            }
        }

        // Палец поднят без распознанного выделения → сброс стартовой точки.
        if !is_down_on && !self.selection_active {
            self.press_pos = None;
        }

        // Палец уехал слишком далеко от точки нажатия → отмена распознавания.
        if let (Some(pos), Some(start)) = (pointer_pos, self.press_pos) {
            if pos.distance(start) > MAX_PRESS_DRIFT * 2.0 {
                self.press_pos = None;
            }
        }

        // Жест завершён (палец поднят) → сброс режима, чтобы следующий
        // long-press мог начаться. Иначе после первого выделения повторное
        // распознавание не срабатывает (`selection_active` застревал в true).
        if !is_down_on && self.selection_active {
            self.selection_active = false;
            self.press_pos = None;
        }

        false
    }

    /// Сбросить при тапе вне выделения / после действия.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Разрешено ли в этом кадре начать распознавание long-press выделения.
///
/// При `is_press_start` (первый кадр удержания: ещё не запомнена позиция), палец
/// опущен (`any_down`) и находится над текстом (`is_over_text`) — да.
///
/// ⚠️ Инвариант: `focused` (поле `TextEdit` в фокусе / IME активен) НЕ блокирует
/// выделение — в фокусе long-press должен работает так же, как вне него (это
/// regression против `&& !is_editing`). Параметр оставлен для ясности контракта,
/// но сознательно не участвует в решении.
pub fn may_start_long_press(
    is_press_start: bool,
    any_down: bool,
    is_over_text: bool,
    _focused: bool,
) -> bool {
    is_press_start && any_down && is_over_text
}

/// Выделить слово под позицией курсора в тексте.
///
/// Логика повторяет `select_word_at` из `text_cursor_state.rs` (патчи egui), но
/// реализована локально в `crates/ui` с нулевой инвазивностью в чужой код. Все
/// хелперы word-boundary — приватные в этом модуле.
pub(crate) fn select_word_at(text: &str, ccursor: CCursor) -> CCursorRange {
    if text.is_empty() {
        return CCursorRange::one(ccursor);
    }

    let line_start = find_line_start(text, ccursor);
    let line_end = ccursor_next_line(text, line_start);

    let line_range = line_start.index..line_end.index;
    let current_line_text = slice_char_range(text, line_range.clone());

    let relative_idx = ccursor.index - line_start.index;
    let relative_ccursor = CCursor::new(relative_idx);

    let min = ccursor_previous_word(current_line_text, relative_ccursor);
    let max = ccursor_next_word(current_line_text, relative_ccursor);

    CCursorRange::two(
        CCursor::new(line_start.index + min.index),
        CCursor::new(line_start.index + max.index),
    )
}

fn ccursor_next_word(text: &str, ccursor: CCursor) -> CCursor {
    CCursor {
        index: next_word_boundary_char_index(text, ccursor.index),
        prefer_next_row: false,
    }
}

fn ccursor_previous_word(text: &str, ccursor: CCursor) -> CCursor {
    let num_chars = CharIndex(text.chars().count());
    let reversed: String = text.graphemes(true).rev().collect();
    let boundary = next_word_boundary_char_index(&reversed, num_chars - ccursor.index);
    CCursor {
        index: num_chars - boundary.min(num_chars),
        prefer_next_row: true,
    }
}

fn ccursor_next_line(text: &str, ccursor: CCursor) -> CCursor {
    CCursor {
        index: next_line_boundary_char_index(text.chars(), ccursor.index),
        prefer_next_row: false,
    }
}

/// Следующая граница строки (перенос). Считает символы до смены «linebreak/non-linebreak».
fn next_line_boundary_char_index(
    it: impl Iterator<Item = char>,
    mut index: CharIndex,
) -> CharIndex {
    let mut it = it.skip(index.0);
    if let Some(_first) = it.next() {
        index += 1;
        if let Some(second) = it.next() {
            index += 1;
            for next in it {
                if is_linebreak(next) != is_linebreak(second) {
                    break;
                }
                index += 1;
            }
        }
    }
    index
}

/// Следующая граница слова (word boundary). `.` считается границей (как на Mac).
fn next_word_boundary_char_index(text: &str, cursor_ci: CharIndex) -> CharIndex {
    let mut current_char_idx = CharIndex::ZERO;

    for (_word_byte_index, word) in text.split_word_bound_indices() {
        let word_ci = current_char_idx;

        let mut word_char_count = 0;
        for chr in word.chars() {
            let dot_ci = word_ci + word_char_count;
            if chr == '.' && cursor_ci < dot_ci {
                return dot_ci;
            }
            word_char_count += 1;
        }

        if cursor_ci < word_ci && !all_word_chars(word) {
            return word_ci;
        }

        current_char_idx += word_char_count;
    }

    current_char_idx
}

fn all_word_chars(text: &str) -> bool {
    text.chars().all(is_word_char)
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn is_linebreak(c: char) -> bool {
    c == '\r' || c == '\n'
}

fn find_line_start(text: &str, current_index: CCursor) -> CCursor {
    let byte_idx = byte_index_from_char_index(text, current_index.index);
    let text_before = &text[..byte_idx];
    if let Some(last_newline_byte) = text_before.rfind('\n') {
        let char_idx = char_index_from_byte_index(text, last_newline_byte + 1);
        CCursor::new(char_idx)
    } else {
        CCursor::new(0)
    }
}

fn byte_index_from_char_index(s: &str, char_index: CharIndex) -> usize {
    for (ci, (bi, _)) in s.char_indices().enumerate() {
        if ci == char_index.0 {
            return bi;
        }
    }
    s.len()
}

fn char_index_from_byte_index(input: &str, byte_index: usize) -> CharIndex {
    for (ci, (bi, _)) in input.char_indices().enumerate() {
        if bi == byte_index {
            return CharIndex(ci);
        }
    }
    // за границей (или не на границе char) — общее число символов
    CharIndex(input.chars().count())
}

fn slice_char_range(s: &str, char_range: std::ops::Range<CharIndex>) -> &str {
    assert!(
        char_range.start <= char_range.end,
        "Invalid range, start must be <= end"
    );
    let start_byte = byte_index_from_char_index(s, char_range.start);
    let end_byte = byte_index_from_char_index(s, char_range.end);
    &s[start_byte..end_byte]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range_lo_hi(r: CCursorRange) -> (usize, usize) {
        let sorted = r.as_sorted_char_range();
        (sorted.start.0, sorted.end.0)
    }

    // ── LongPressState::update_raw ──

    #[test]
    fn long_press_detected_at_400ms() {
        let mut s = LongPressState::default();
        let pos = Pos2::new(10.0, 20.0);
        // палец опустился — запоминаем время/позицию
        assert!(!s.update_raw(0.0, true, Some(pos)));
        // удержано ровно 400мс без дрейфа → распознано
        assert!(s.update_raw(0.4, true, Some(pos)));
    }

    #[test]
    fn long_press_cancelled_by_drift() {
        let mut s = LongPressState::default();
        let pos = Pos2::new(10.0, 20.0);
        s.update_raw(0.0, true, Some(pos));
        // палец уехал на 13px (> MAX_PRESS_DRIFT) до распознавания → отмена
        assert!(!s.update_raw(0.5, true, Some(Pos2::new(23.0, 20.0))));
    }

    #[test]
    fn long_press_single_trigger() {
        let mut s = LongPressState::default();
        let pos = Pos2::new(10.0, 20.0);
        s.update_raw(0.0, true, Some(pos));
        assert!(s.update_raw(0.4, true, Some(pos)));
        // после срабатывания не триггерится повторно (selection_active)
        s.press_time = 0.0; // эмулируем новое длительное удержание после срабатывания
        assert!(!s.update_raw(0.5, true, Some(pos)));
    }

    #[test]
    fn reset_on_release() {
        let mut s = LongPressState::default();
        let pos = Pos2::new(10.0, 20.0);
        s.update_raw(0.0, true, Some(pos));
        // до порога отпустили → сброс, повторное нажатие начинается заново
        assert!(!s.update_raw(0.2, false, None));
        assert!(!s.update_raw(0.2, true, Some(pos)));
        // повторное удержание 500мс (>= 400мс) → распознано
        assert!(s.update_raw(0.7, true, Some(pos)));
    }

    #[test]
    fn below_threshold_no_trigger() {
        let mut s = LongPressState::default();
        let pos = Pos2::new(10.0, 20.0);
        s.update_raw(0.0, true, Some(pos));
        // 399мс — ещё не распознано
        assert!(!s.update_raw(0.399, true, Some(pos)));
    }

    // Regression: после первого выделения и отпускания повторный long-press
    // в том же поле должен снова распознаваться.
    #[test]
    fn long_press_can_retrigger_after_release() {
        let mut s = LongPressState::default();
        let pos = Pos2::new(10.0, 20.0);
        // первое выделение
        s.update_raw(0.0, true, Some(pos));
        assert!(s.update_raw(0.4, true, Some(pos)));
        // отпустили палец
        s.update_raw(0.5, false, None);
        // новое удержание → снова распознаёт
        s.update_raw(0.5, true, Some(pos));
        assert!(s.update_raw(0.9, true, Some(pos)));
    }

    // ── may_start_long_press ──

    /// Regression: long-press должен стартовать и когда `TextEdit` в фокусе (IME).
    #[test]
    fn may_start_long_press_allows_focused_editable() {
        // `focused = true` НЕ должен блокировать старт (было `&& !is_editing`).
        assert!(may_start_long_press(true, true, true, true));
        assert!(may_start_long_press(true, true, true, false));
    }

    #[test]
    fn may_start_long_press_requires_press_down_and_text() {
        // press уже начат (не первый кадр) — не старт
        assert!(!may_start_long_press(false, true, true, true));
        // палец не нажат
        assert!(!may_start_long_press(true, false, true, true));
        // палец вне текста
        assert!(!may_start_long_press(true, true, false, true));
    }

    // ── select_word_at ──

    #[test]
    fn select_word_basic() {
        // "hello world", cursor на 'l'(2) → "hello" (0..5)
        let r = select_word_at("hello world", CCursor::new(2));
        assert_eq!(range_lo_hi(r), (0, 5));

        // cursor на 'o' в "world"(8) → "world" (6..11)
        let r = select_word_at("hello world", CCursor::new(8));
        assert_eq!(range_lo_hi(r), (6, 11));
    }

    #[test]
    fn select_word_punctuation() {
        // "foo,bar", cursor на 'a'(5) → "bar" (4..7)
        let r = select_word_at("foo,bar", CCursor::new(5));
        assert_eq!(range_lo_hi(r), (4, 7));
    }

    #[test]
    fn select_word_empty_text_no_panic() {
        let r = select_word_at("", CCursor::new(0));
        let (lo, hi) = range_lo_hi(r);
        assert_eq!(lo, hi); // пустой диапазон
    }

    #[test]
    fn select_word_emoji() {
        // Графемное слово с эмодзи: элементы считаются word-boundary,
        // совпадает с поведением egui (см. text_cursor_state::test).
        let r = select_word_at("❤️👍 skvěla", CCursor::new(2));
        // cursor на части эмодзи-графемы: ожидаем диапазон до разделителя-пробела
        let (lo, hi) = range_lo_hi(r);
        assert!(
            lo <= 2 && hi >= 2,
            "word ({lo}..{hi}) должен покрывать cursor=2"
        );
    }

    #[test]
    fn select_word_on_whitespace_advances() {
        // cursor на пробеле после "hello" → привязан к следующему/предыдущему слову
        let r = select_word_at("hello world", CCursor::new(5));
        let (lo, hi) = range_lo_hi(r);
        // пробел ещё не считается словом: диапазон либо пустой, либо вокруг соседнего слова
        assert!(lo <= 5 && 5 <= hi, "диапазон ({lo}..{hi}) вокруг cursor=5");
    }
}
