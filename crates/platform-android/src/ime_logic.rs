//! Входной тип команд IME и утилита перевода UTF-16 в char-индексы.
//!
//! Хост-совместимый модуль (без `cfg(target_os = "android")`), чтобы команды
//! из Kotlin `InputConnection` и маппинг позиций были покрыты юнит-тестами на
//! хосте. Собственно редьюсер команд → буферные операции вынесен в
//! [`crate::ime_service`]; здесь остаются только типы/утилиты, которыми
//! пользуются `ime_jni` (JNI-мост) и `ime_service::translate_legacy`.

/// IME-команда, пришедшая из Kotlin `InputConnection` через JNI.
///
/// Генерируется на главном Java-потоке (в `EguiImeView` InputConnection),
/// доставляется в главный цикл через потокобезопасную очередь `PlatformState`,
/// затем конвертируется в команду [`crate::ime_service`] (см.
/// `ime_service::translate_legacy`) и применяется редьюсером.
#[derive(Debug, Clone, PartialEq)]
pub enum ImeCmd {
    /// `commitText(text)` — финальный текст (как правило, одна строка/символ).
    Commit(String),
    /// `setComposingText(text)` — промежуточный предредактируемый текст (composition).
    Composing(String),
    /// `performEditorAction(Next)` — перейти к следующему TextEdit.
    Next,
    /// `performEditorAction(Done/Search/Go)` — завершить редактирование, скрыть клавиатуру.
    Done,
    /// `deleteSurroundingText(before, after)` — удалить текст вокруг курсора.
    DeleteSurrounding { before: i32, after: i32 },
    /// `setComposingText` с диапазоном (start..end). Поля — char-индексы в
    /// существующем тексте буфера (уже переведены из UTF-16 на платформе).
    ComposingRange {
        text: String,
        start_char: usize,
        end_char: usize,
    },
    /// `setSelection(start, end)` — пользователь переставил курсор/выделение.
    /// В текущей модели не влияет на композицию (`translate_legacy` пропускает).
    SetSelection { start: i32, end: i32 },
    /// `beginBatchEdit()` — начало пакетной операции IME.
    BeginBatchEdit,
    /// `endBatchEdit()` — завершение пакетной операции.
    EndBatchEdit,
    /// `performPrivateCommand(action)` — приватная команда IME (Gboard и др.).
    PrivateCommand(String),
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
}
