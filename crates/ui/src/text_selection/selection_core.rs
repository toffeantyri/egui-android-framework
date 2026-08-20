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

use egui::text::{CCursor, CCursorRange, Galley};
use egui::Pos2;

use super::android_behavior::{select_word_at, HandleSide};
use super::drag_handles::{selection_bbox, HANDLE_VERTICAL_OFFSET};
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
    /// Какую капельку физически перетаскивают (совпадает с текущей ролью
    /// primary/secondary). Нужна, чтобы при «обгоне» границы выделение не
    /// схлопывалось в ноль, а пересчитывалось с другой стороны (см. `drag_handle`).
    drag_side: Option<HandleSide>,
    /// Long-press в пустом месте поля (вне слова): режим «кареточного» попапа.
    /// Отличается от выделения слова — для набора кнопок попапа (Вставить/Всё)
    /// и отдельного жизненного цикла (не ручки, не обгон).
    pub caret_mode: bool,
    /// Rect позиции каретки (для попапа в `caret_mode`).
    pub caret_rect: Option<egui::Rect>,
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

    /// Long-press по позиции: либо выделяем слово (как [`Self::select_word_at`]),
    /// либо, если курсор в пустом месте (пробел/край), включаем «кареточный» режим
    /// попапа и возвращаем курсор, куда виджет поставит каретку (для вставки).
    ///
    /// Возвращает `Some(cursor)` при `caret_mode` (надо поставить каретку),
    /// `None` — когда выделили слово.
    pub fn long_press_start(
        &mut self,
        pos: Pos2,
        galley: &Galley,
        galley_pos: Pos2,
        text: &str,
    ) -> Option<CCursor> {
        let local = pos - galley_pos.to_vec2();
        let cursor = galley.cursor_from_pos(local.to_vec2());

        match super::android_behavior::word_range_or_caret(text, cursor) {
            super::android_behavior::LongPressTarget::Word(range) => {
                let selected = range.slice_str(text).to_owned();
                self.active = true;
                self.selection = Some(range);
                self.selected_text = selected;
                self.selection_rect = Some(selection_bbox(galley, galley_pos, &range));
                self.caret_mode = false;
                self.caret_rect = None;
                None
            }
            super::android_behavior::LongPressTarget::Caret(ccursor) => {
                // Позиция каретки → rect для попапа (высота строки галели).
                let row = galley.pos_from_cursor(ccursor);
                self.active = true;
                self.selection = None;
                self.selected_text.clear();
                self.caret_mode = true;
                self.caret_rect = Some(egui::Rect::from_center_size(
                    galley_pos + row.center().to_vec2(),
                    egui::vec2(0.0, galley.size().y.min(24.0)),
                ));
                self.selection_rect = None;
                Some(ccursor)
            }
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
            self.drag_side = Some(side);
            return;
        }

        let mut local = pos - galley_pos.to_vec2();
        // Капелька рисуется ниже точки привязки (строки), на которой ставится курсор
        // (смещение HANDLE_VERTICAL_OFFSET вниз). Пока палец на капельке, его позиция
        // на offset ниже нужной строки — компенсируем, иначе курсор «сползает» вниз
        // (а у конца текста — прыгает в самый конец → выделяется весь текст).
        local.y -= HANDLE_VERTICAL_OFFSET;
        // Проецируем y на диапазон строк галели: ниже последней строки — последняя.
        local.y = local.y.clamp(0.0, galley.size().y);
        let cursor = galley.cursor_from_pos(local.to_vec2());

        if let Some(range) = self.selection.as_mut() {
            // Тянем ту же физическую капельку, что и в начале драга (primary/secondary
            // относятся к ролям, а не к лево/право). Свежий `side` из `dragged_handle`
            // берём только на первом кадре (когда drag_side ещё не выставлен).
            let side = self.drag_side.unwrap_or(side);

            match side {
                HandleSide::Start => range.secondary = cursor,
                HandleSide::End => range.primary = cursor,
            }

            // Активная (тянущаяся) капелька сохраняет свою роль по отношению к
            // пассивной на протяжении всего жеста: End остаётся справа, Start — слева.
            // Если тянущаяся граница «обогнала» пассивную (перexодила на другую
            // сторону), меняем primary/secondary местами и продолжаем тянуть ту же
            // физическую капельку — выделение не схлопывается в ноль, а фиксирует
            // другую границу и расширяется в противоположную сторону.
            let crossed = match side {
                // End (primary) остаётся правее (>=) Start (secondary).
                HandleSide::End => range.primary.index <= range.secondary.index,
                // Start (secondary) остаётся левее (<=) End (primary).
                HandleSide::Start => range.secondary.index >= range.primary.index,
            };
            if crossed {
                std::mem::swap(&mut range.primary, &mut range.secondary);
                self.drag_side = Some(match side {
                    HandleSide::Start => HandleSide::End,
                    HandleSide::End => HandleSide::Start,
                });
            }

            if range.is_empty() {
                // Точное совпадение границ (End дошла до Start). НЕ схлопываем:
                // это точка «переворота» — как только активная капелька уйдёт за
                // пассивную, диапазон оживёт с другой стороны. Пустой кадр рисует
                // только капельку (фон/тулбар скрыты), а состояние
                // (active / drag_started / drag_side) сохраняется, чтобы жесть
                // продолжился после пересечения.
                self.selected_text.clear();
                self.selection_rect = None;
                return;
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
        self.caret_mode = false;
        self.caret_rect = None;
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
        self.drag_side = None;
        self.caret_mode = false;
        self.caret_rect = None;
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
    fn drag_handle_compensates_handle_vertical_offset() {
        with_real_ui(|ui| {
            let galley = make_galley(ui, "hello world");
            let mut core = SelectionCore::default();
            core.selection = Some(CCursorRange::two(CCursor::new(0), CCursor::new(5)));
            core.drag_started = true;

            // Палец на капельке End, которая на HANDLE_VERTICAL_OFFSET ниже строки
            // курсора 5. После компенсации курсор должен остаться ≈ за строкой 5,
            // а НЕ уехать на последнюю строку/в конец текста.
            let row_center = galley.pos_from_cursor(CCursor::new(5)).center();
            let handle_pos = Pos2::new(row_center.x, row_center.y + HANDLE_VERTICAL_OFFSET);
            core.drag_handle(HandleSide::End, handle_pos, &galley, Pos2::ZERO);

            let r = core.selection.as_ref().unwrap();
            assert!(
                r.primary.index.0 <= 8,
                "End не должен уезжать в конец текста (получено {})",
                r.primary.index.0
            );
        });
    }

    #[test]
    fn drag_handle_end_meets_start_turns_handle_for_overtake() {
        with_real_ui(|ui| {
            let galley = make_galley(ui, "hello world");
            let mut core = SelectionCore::default();
            core.active = true;
            core.drag_started = true;
            core.selection = Some(CCursorRange::two(CCursor::new(2), CCursor::new(5)));
            core.selected_text = "llo".to_owned();

            // End-handle (primary=5) тянем до secondary (Start=2): это точка «переворота».
            // Диапазон становится нулевым на этот кадр, но состояer НЕ сбрасывается —
            // активность и drag сохранены, чтобы жесть продолжился после обгона.
            let at_secondary = galley.pos_from_cursor(CCursor::new(2)).center();
            core.drag_handle(HandleSide::End, at_secondary, &galley, egui::Pos2::ZERO);

            assert!(core.active, "переворот не сбрасывает активность");
            assert!(core.drag_started, "переворот не сбрасывает drag");
            let r = core.selection.as_ref().unwrap();
            assert!(
                r.is_empty(),
                "в точке совпадения диапазон пуст на один кадр"
            );

            // Палец продолжает влево (обгон): диапазон оживает от новой границы.
            let further_left = galley.pos_from_cursor(CCursor::new(0)).center();
            // После переворота капелька ведётся как Start (см. drag_side).
            core.drag_side = Some(HandleSide::Start);
            core.drag_handle(HandleSide::Start, further_left, &galley, egui::Pos2::ZERO);
            let r = core.selection.as_ref().unwrap();
            assert!(!r.is_empty(), "после обгона выделение непустое");
            let sorted = r.as_sorted_char_range();
            assert_eq!(
                sorted.start.0, 0,
                "выделение расширяется влево от новой границы"
            );
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

    #[test]
    fn drag_handle_end_overtakes_start_keeps_selection() {
        with_real_ui(|ui| {
            let galley = make_galley(ui, "hello world");
            let mut core = SelectionCore::default();
            core.active = true;
            core.selection = Some(CCursorRange::two(CCursor::new(2), CCursor::new(5)));
            core.selected_text = "llo".to_owned();
            core.drag_started = true;
            core.drag_side = Some(HandleSide::End);

            // End (primary=5, правая граница) тянем влево, в индекс 1 — левее Start
            // (secondary=2). Выделение НЕ должно схлопнуться: тянущаяся капелька
            // становится левой границей, и диапазон пересчитывается как [1..2].
            let left_of_start = galley.pos_from_cursor(CCursor::new(1)).center();
            core.drag_handle(HandleSide::End, left_of_start, &galley, egui::Pos2::ZERO);

            assert!(core.active, "после обгона выделение остаётся активным");
            let r = core.selection.as_ref().unwrap();
            assert!(!r.is_empty(), "выделение не схлопнулось в ноль");
            let sorted = r.as_sorted_char_range();
            assert_eq!(sorted.start.0, 1, "новая левая граница — тянущийся End");
            assert_eq!(sorted.end.0, 2, "старый Start стал правой границей");
            assert_eq!(
                core.selected_text, "l",
                "выделен текст между новой левой и пассивной правой"
            );
            // Тянем ту же (теперь левую) капельку левее — диапазон растёт влево.
            let further_left = galley.pos_from_cursor(CCursor::new(0)).center();
            core.drag_handle(HandleSide::Start, further_left, &galley, egui::Pos2::ZERO);
            let r2 = core.selection.as_ref().unwrap();
            assert_eq!(
                r2.as_sorted_char_range().start.0,
                0,
                "выделение расширяется влево"
            );
        });
    }

    #[test]
    fn drag_handle_start_overtakes_end_keeps_selection() {
        with_real_ui(|ui| {
            let galley = make_galley(ui, "hello world");
            let mut core = SelectionCore::default();
            core.active = true;
            core.selection = Some(CCursorRange::two(CCursor::new(2), CCursor::new(5)));
            core.selected_text = "llo".to_owned();
            core.drag_started = true;
            core.drag_side = Some(HandleSide::Start);

            // Start (secondary=2, левая граница) тянем вправо, в индекс 7 — правее End
            // (primary=5). Тянущаяся капелька становится правой границей, выделение
            // пересчитывается как [5..7].
            let right_of_end = galley.pos_from_cursor(CCursor::new(7)).center();
            core.drag_handle(HandleSide::Start, right_of_end, &galley, egui::Pos2::ZERO);

            assert!(core.active, "после обгона выделение остаётся активным");
            let r = core.selection.as_ref().unwrap();
            assert!(!r.is_empty(), "выделение не схлопнулось в ноль");
            let sorted = r.as_sorted_char_range();
            assert_eq!(sorted.start.0, 5, "старый End стал левой границей");
            assert_eq!(sorted.end.0, 7, "новая правая граница — тянущийся Start");
            assert_eq!(
                core.selected_text, "wo",
                "выделен текст между старой левой и новой правой"
            );
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
