//! Чистая IME-логика: преобразование `ImeCmd` → `egui::Event`.
//!
//! Вынесена в **хост-совместимый** модуль (без `cfg(target_os = "android")`),
//! чтобы преобразование и batch-буферизация были покрыты юнит-тестами,
//! запускаемыми на хосте (`cargo test`). Android-специфичный ввод/FFI
//! (`android_activity`, JNI) остаётся в `input.rs` / `ime_jni.rs`.
//!
//! Модель данных:
//! - [`ImeCmd`] — команда из Kotlin `InputConnection` (через JNI).
//! - [`ImeOutcome`] — управляющий запрос для главного цикла (`Next`/`Done`).
//! - [`ImeBuffer`] — накопитель egui-событий с поддержкой batch-операций.
//!
//! `ImeCmd` и `ImeOutcome` объявлены здесь, а не в `event.rs`, потому что
//! `event.rs` целиком под `cfg(target_os = "android")`, что отключает его на
//! хосте и не даёт писать тесты.

/// IME-команда, пришедшая из Kotlin `InputConnection` через JNI.
///
/// Эти команды генерируются на главном Java-потоке (в `EguiImeView`
/// InputConnection) и доставляются в главный цикл через потокобезопасную
/// очередь в `PlatformState`. В цикле они конвертируются в `egui::Event`.
///
/// Только здесь получается контент для ввода — никакого использования
/// `InputEvent::TextEvent` / `setTextInputState` / `textInputState()`.
#[derive(Debug, Clone, PartialEq)]
pub enum ImeCmd {
    /// `commitText(text)` — финальный текст (как правило, одна строка/символ).
    /// Конвертируется в `egui::ImeEvent::Commit(text)` — финализирует IME-композицию.
    Commit(String),
    /// `setComposingText(text)` — промежуточный предредактируемый текст (composition).
    /// Используется для отображения preedit, не ломая буфер.
    Composing(String),
    /// `performEditorAction(Next)` — перейти к следующему TextEdit.
    Next,
    /// `performEditorAction(Done/Search/Go)` — завершить редактирование, скрыть клавиатуру.
    Done,
    /// `deleteSurroundingText(before, after)` — удалить текст вокруг курсора.
    DeleteSurrounding { before: i32, after: i32 },
    /// `setComposingText` с диапазоном (start..end) в текущей строке.
    ComposingRange { text: String, start: i32, end: i32 },
    /// `setSelection(start, end)` — пользователь переставил курсор/выделение.
    /// Позиции — в UTF-16 code units (как Android). В текущей версии полностью
    /// применить к egui-курсору сложно; инкапсулируется как no-op (см.
    /// `process_ime_cmd`).
    SetSelection { start: i32, end: i32 },
    /// `beginBatchEdit()` — начало пакетной операции IME. Пока буфер открыт,
    /// текстовые события буферизуются, а не попадают в `pending`.
    BeginBatchEdit,
    /// `endBatchEdit()` — завершение пакетной операции. Накопленные события
    /// переносятся в `pending`.
    EndBatchEdit,
    /// `performPrivateCommand(action)` — приватная команда IME (Gboard, Samsung и т.д.).
    /// Не обрабатывается функционально (egui не имеет API для private-команд),
    /// только логируется для диагностики проблем с клавиатурой.
    PrivateCommand(String),
}

/// Управляющий запрос от IME, требующий действия главного цикла (вне egui-событий).
///
/// Возвращается из [`process_ime_cmd`] для `performEditorAction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImeOutcome {
    /// `IME_ACTION_NEXT` — пользователь хочет перейти к следующему TextEdit.
    Next,
    /// `IME_ACTION_DONE/SEARCH/GO` — завершить редактирование, скрыть клавиатуру.
    Done,
    /// Обычное текстовое событие — специального действия не требуется.
    None,
}

/// Накопитель egui-событий от IME с поддержкой batch-операций.
///
/// Аналог части `InputState` (поля `ime_pending`, `ime_batch_depth`,
/// `ime_batch_buffer`), но без android-зависимостей, чтобы `process_ime_cmd`
/// можно было тестировать на хосте.
#[derive(Debug, Default)]
pub struct ImeBuffer {
    /// Очередь событий на доставку в egui (двухкадровая).
    pub pending: Vec<egui::Event>,
    /// Глубина вложенности batch-операций IME (`beginBatchEdit`/`endBatchEdit`).
    /// Пока `batch_depth > 0`, события буферизуются в `batch_buffer`.
    pub batch_depth: u32,
    /// Буфер для накопления IME-событий внутри batch-операции.
    pub batch_buffer: Vec<egui::Event>,
}

impl ImeBuffer {
    /// Создать пустой буфер.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Преобразовать IME-команду (из Kotlin InputConnection) в egui-события.
///
/// Возвращает [`ImeOutcome`] для управляющих действий (`Next`/`Done`), которые
/// главный цикл (`loop.rs`) обрабатывает отдельно. Текстовые команды
/// (`Commit`, `Composing`, `DeleteSurrounding`) кладутся в `buffer.pending`
/// (или `buffer.batch_buffer`, если активна batch-операция) и доставляются
/// в egui на **следующем** кадре (двухкадровая доставка).
///
/// # Содержительное преобразование
///
/// - `ImeCmd::Commit(text)` → `egui::Event::Ime(ImeEvent::Commit(text))`
/// - `ImeCmd::Composing(text)` → `egui::Event::Ime(ImeEvent::Preedit)`
/// - `ImeCmd::DeleteSurrounding{before,after}` → pairs `Event::Key` (Backspace / Delete)
///
/// Никакого `InputEvent::TextEvent` / `setTextInputState` — только InputConnection.
pub fn process_ime_cmd(buffer: &mut ImeBuffer, cmd: ImeCmd) -> ImeOutcome {
    use egui::Key;

    // Если активен batch (batch_depth > 0), события буферизуются в
    // `batch_buffer` и выгружаются только при `endBatchEdit`.
    let pending = if buffer.batch_depth > 0 {
        &mut buffer.batch_buffer
    } else {
        &mut buffer.pending
    };

    match cmd {
        ImeCmd::Commit(text) => {
            // Commit финализирует IME-композицию: очищает активный preedit и
            // вставляет текст в позицию курсора (как IME-финал). Использование
            // `ImeEvent::Commit` (а не `Event::Text`) корректно завершает preedit,
            // когда перед commitText пришли setComposingText-кадры.
            log::info!(
                "IME: commitText -> {:?} (ImeEvent::Commit, next frame)",
                text
            );
            pending.push(egui::Event::Ime(egui::ImeEvent::Commit(text)));
            ImeOutcome::None
        }
        ImeCmd::Composing(text) => {
            // Предредактируемый текст composition. Если строка пустая — IME
            // завершил preedit (active_range = None).
            log::info!("IME: setComposingText -> {:?} (Preedit, next frame)", text);
            let active_range_chars = if text.is_empty() {
                None
            } else {
                Some(0..text.chars().count())
            };
            pending.push(egui::Event::Ime(egui::ImeEvent::Preedit {
                text,
                active_range_chars,
            }));
            ImeOutcome::None
        }
        ImeCmd::ComposingRange { text, start, end } => {
            log::info!("IME: setComposingText -> {:?} (Preedit, next frame)", text);
            let active_range_chars = if text.is_empty() {
                None
            } else {
                // start/end в UTF-16 code units (из ComposingRange) — пересчитываем
                // в char-индексы для egui `active_range_chars`.
                let char_start = utf16_offset_to_char_index(&text, start as usize);
                let char_end = utf16_offset_to_char_index(&text, end as usize);
                Some(char_start..char_end)
            };
            pending.push(egui::Event::Ime(egui::ImeEvent::Preedit {
                text,
                active_range_chars,
            }));
            ImeOutcome::None
        }
        ImeCmd::Next => {
            log::info!("IME: IME_ACTION_NEXT");
            ImeOutcome::Next
        }
        ImeCmd::Done => {
            log::info!("IME: IME_ACTION_DONE");
            ImeOutcome::Done
        }
        ImeCmd::DeleteSurrounding { before, after } => {
            log::info!(
                "IME: deleteSurroundingText before={} after={} (Backspace/Delete, next frame)",
                before,
                after
            );
            // before → Backspace, after → Delete. Ограничиваем разумным числом.
            let before = before.clamp(0, 4);
            let after = after.clamp(0, 4);
            for _ in 0..before {
                pending.push(egui::Event::Key {
                    key: Key::Backspace,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::default(),
                });
            }
            for _ in 0..after {
                pending.push(egui::Event::Key {
                    key: Key::Delete,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::default(),
                });
            }
            ImeOutcome::None
        }
        ImeCmd::SetSelection { start, end } => {
            // Пользователь переставил курсор/выделение в IME. Полноценное
            // применение к egui-курсору требует обратного маппинга UTF-16 ->
            // символьные индексы и программной установки cursor range — в
            // текущей версии управляемо источником (egui сам обрабатывает
            // тапы/стрелки). Логируем и ничего не меняем.
            log::info!("IME: setSelection {start}..{end} (no-op, курсором управляет egui)");
            ImeOutcome::None
        }
        ImeCmd::BeginBatchEdit => {
            buffer.batch_depth += 1;
            log::debug!("IME: beginBatchEdit depth={}", buffer.batch_depth);
            ImeOutcome::None
        }
        ImeCmd::EndBatchEdit => {
            if buffer.batch_depth > 0 {
                buffer.batch_depth -= 1;
            }
            log::debug!("IME: endBatchEdit depth={}", buffer.batch_depth);
            if buffer.batch_depth == 0 {
                let buffered: Vec<egui::Event> = std::mem::take(&mut buffer.batch_buffer);
                if !buffered.is_empty() {
                    log::debug!("IME: endBatchEdit — выгружено {} событий", buffered.len());
                    buffer.pending.extend(buffered);
                }
            }
            ImeOutcome::None
        }
        ImeCmd::PrivateCommand(action) => {
            // Приватная команда IME (action — нестандартизированная строка Gboard,
            // Samsung и др.). egui не имеет API для private-команд, поэтому не
            // порождаем egui-событие и не меняем состояние буфера. Только фиксируем
            // для диагностики проблем с клавиатурой.
            log::info!("IME: performPrivateCommand action={:?}", action);
            ImeOutcome::None
        }
    }
}

/// Пересчитать UTF-16 offset в char-индекс (Unicode scalar value index).
///
/// Android IME передаёт позиции в UTF-16 code units (16-битные слова).
/// egui `active_range_chars` ожидает char-индексы (Unicode scalar values).
///
/// Для BMP-символов (кириллица, латиница, цифры) они совпадают.
/// Для non-BMP (эмодзи, некоторые иероглифы) UTF-16 единица = 2 суррогата,
/// поэтому char-индекс будет меньше.
pub fn utf16_offset_to_char_index(text: &str, utf16_offset: usize) -> usize {
    text.chars()
        .scan(0usize, |utf16_pos, ch| {
            let unit_len = ch.len_utf16();
            let current = *utf16_pos;
            *utf16_pos += unit_len;
            if current >= utf16_offset {
                None
            } else {
                Some(())
            }
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Event;

    /// Проверка: BMP — UTF-16 offset == char index.
    #[test]
    fn utf16_offset_bmp_chars_match() {
        assert_eq!(utf16_offset_to_char_index("hello", 3), 3);
        assert_eq!(utf16_offset_to_char_index("привет", 4), 4);
    }

    /// Проверка: offset 0 всегда 0 (даже для эмодзи).
    #[test]
    fn utf16_offset_zero_always_zero() {
        assert_eq!(utf16_offset_to_char_index("🎉🎉", 0), 0);
        assert_eq!(utf16_offset_to_char_index("", 0), 0);
    }

    /// Проверка: offset за пределами → длина в символах.
    #[test]
    fn utf16_offset_beyond_text_maps_to_len() {
        assert_eq!(utf16_offset_to_char_index("hi", 100), 2);
        assert_eq!(utf16_offset_to_char_index("", 5), 0);
    }

    /// Проверка: non-BMP (эмодзи) — char index меньше UTF-16 offset.
    #[test]
    fn utf16_offset_emoji_reduces_char_index() {
        // "a🎉b": UTF-16 = [a][high surrogate][low surrogate][b] = 4 units.
        // char index конца строки = 3 (a, emoji, b).
        assert_eq!(utf16_offset_to_char_index("a🎉b", 4), 3);
        // после 'a' (1 unit) → 1 символ.
        assert_eq!(utf16_offset_to_char_index("a🎉b", 1), 1);
    }

    /// Проверка: commitText без batch → напрямую в pending.
    #[test]
    fn commit_goes_directly_to_pending_without_batch() {
        let mut buf = ImeBuffer::new();
        let outcome = process_ime_cmd(&mut buf, ImeCmd::Commit("тест".into()));
        assert_eq!(outcome, ImeOutcome::None);
        assert_eq!(buf.pending.len(), 1);
        assert!(buf.batch_buffer.is_empty());
        assert_eq!(buf.batch_depth, 0);
        let ev = &buf.pending[0];
        assert!(matches!(ev, Event::Ime(egui::ImeEvent::Commit(_))));
    }

    /// Проверка: Next → ImeOutcome::Next, событий нет.
    #[test]
    fn next_returns_next_outcome() {
        let mut buf = ImeBuffer::new();
        let outcome = process_ime_cmd(&mut buf, ImeCmd::Next);
        assert_eq!(outcome, ImeOutcome::Next);
        assert!(buf.pending.is_empty());
    }

    /// Проверка: Done → ImeOutcome::Done.
    #[test]
    fn done_returns_done_outcome() {
        let mut buf = ImeBuffer::new();
        let outcome = process_ime_cmd(&mut buf, ImeCmd::Done);
        assert_eq!(outcome, ImeOutcome::Done);
        assert!(buf.pending.is_empty());
    }

    /// Проверка: deleteSurrounding(before) → N Backspace-событий.
    #[test]
    fn delete_surrounding_before_emits_backspaces() {
        let mut buf = ImeBuffer::new();
        process_ime_cmd(
            &mut buf,
            ImeCmd::DeleteSurrounding {
                before: 2,
                after: 0,
            },
        );
        assert_eq!(buf.pending.len(), 2);
        assert!(buf.pending.iter().all(|e| {
            matches!(
                e,
                Event::Key {
                    key: egui::Key::Backspace,
                    pressed: true,
                    ..
                }
            )
        }));
    }

    /// Проверка: deleteSurrounding(after) → N Delete-событий.
    #[test]
    fn delete_surrounding_after_emits_deletes() {
        let mut buf = ImeBuffer::new();
        process_ime_cmd(
            &mut buf,
            ImeCmd::DeleteSurrounding {
                before: 0,
                after: 1,
            },
        );
        assert_eq!(buf.pending.len(), 1);
        assert!(matches!(
            buf.pending[0],
            Event::Key {
                key: egui::Key::Delete,
                pressed: true,
                ..
            }
        ));
    }

    /// Проверка: batch-операция буферизует события и выгружает на end.
    #[test]
    fn batch_edit_buffers_and_flushes() {
        let mut buf = ImeBuffer::new();

        // begin batch
        process_ime_cmd(&mut buf, ImeCmd::BeginBatchEdit);
        assert_eq!(buf.batch_depth, 1);

        // commit внутри batch → в batch_buffer, не в pending
        process_ime_cmd(&mut buf, ImeCmd::Commit("hello".into()));
        assert_eq!(buf.batch_buffer.len(), 1);
        assert!(buf.pending.is_empty());

        // composing внутри batch → тоже в буфер
        process_ime_cmd(&mut buf, ImeCmd::Composing("w".into()));
        assert_eq!(buf.batch_buffer.len(), 2);

        // end batch → выгрузка в pending
        process_ime_cmd(&mut buf, ImeCmd::EndBatchEdit);
        assert_eq!(buf.batch_depth, 0);
        assert!(buf.batch_buffer.is_empty());
        assert_eq!(buf.pending.len(), 2);
    }

    /// Проверка: вложенный batch выгружается только при глубине 0.
    #[test]
    fn nested_batch_flushes_only_at_depth_zero() {
        let mut buf = ImeBuffer::new();

        process_ime_cmd(&mut buf, ImeCmd::BeginBatchEdit);
        process_ime_cmd(&mut buf, ImeCmd::BeginBatchEdit);
        assert_eq!(buf.batch_depth, 2);

        process_ime_cmd(&mut buf, ImeCmd::Commit("x".into()));
        assert_eq!(buf.batch_buffer.len(), 1);

        // end внутреннего batch — НЕ выгружает
        process_ime_cmd(&mut buf, ImeCmd::EndBatchEdit);
        assert_eq!(buf.batch_depth, 1);
        assert!(!buf.batch_buffer.is_empty());
        assert!(buf.pending.is_empty());

        // end внешнего batch — выгружает
        process_ime_cmd(&mut buf, ImeCmd::EndBatchEdit);
        assert_eq!(buf.batch_depth, 0);
        assert!(buf.batch_buffer.is_empty());
        assert_eq!(buf.pending.len(), 1);
    }

    /// Проверка: Composing с непустым текстом → Preedit с active_range = 0..len.
    #[test]
    fn composing_sets_active_range_to_full_text() {
        let mut buf = ImeBuffer::new();
        process_ime_cmd(&mut buf, ImeCmd::Composing("пр".into()));
        assert_eq!(buf.pending.len(), 1);
        match &buf.pending[0] {
            Event::Ime(egui::ImeEvent::Preedit {
                text,
                active_range_chars,
            }) => {
                assert_eq!(text, "пр");
                assert_eq!(active_range_chars, &Some(0..2));
            }
            other => panic!("ожидал Preedit, получил {:?}", other),
        }
    }

    /// Проверка: пустой Composing → Preedit с active_range = None (конец composition).
    #[test]
    fn empty_composing_resets_active_range() {
        let mut buf = ImeBuffer::new();
        process_ime_cmd(&mut buf, ImeCmd::Composing(String::new()));
        assert_eq!(buf.pending.len(), 1);
        match &buf.pending[0] {
            Event::Ime(egui::ImeEvent::Preedit {
                active_range_chars, ..
            }) => {
                assert_eq!(active_range_chars, &None);
            }
            other => panic!("ожидал Preedit, получил {:?}", other),
        }
    }

    /// Проверка: SetSelection — no-op (не генерирует egui-событий).
    #[test]
    fn set_selection_is_noop() {
        let mut buf = ImeBuffer::new();
        let outcome = process_ime_cmd(&mut buf, ImeCmd::SetSelection { start: 1, end: 3 });
        assert_eq!(outcome, ImeOutcome::None);
        assert!(buf.pending.is_empty());
        assert!(buf.batch_buffer.is_empty());
    }

    /// Проверка: PrivateCommand — no-op (не генерирует egui-событий, буферы пусты).
    #[test]
    fn private_command_is_noop() {
        let mut buf = ImeBuffer::new();
        let outcome = process_ime_cmd(
            &mut buf,
            ImeCmd::PrivateCommand("com.google.android.inputmethod.latin.emoji".into()),
        );
        assert_eq!(outcome, ImeOutcome::None);
        assert!(buf.pending.is_empty());
        assert_eq!(buf.batch_depth, 0, "batch_depth не должен меняться");
        assert!(buf.batch_buffer.is_empty());
    }

    /// Проверка: PrivateCommand внутри batch тоже no-op — не попадает ни в pending,
    /// ни в batch_buffer, и не влияет на глубину batch.
    #[test]
    fn private_command_is_noop_inside_batch() {
        let mut buf = ImeBuffer::new();
        process_ime_cmd(&mut buf, ImeCmd::BeginBatchEdit);
        let outcome = process_ime_cmd(&mut buf, ImeCmd::PrivateCommand("x".into()));
        assert_eq!(outcome, ImeOutcome::None);
        assert!(buf.pending.is_empty());
        assert!(buf.batch_buffer.is_empty());
        assert_eq!(buf.batch_depth, 1);
    }
}
