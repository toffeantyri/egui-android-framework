/// IME event.
///
/// See <https://docs.rs/winit/latest/winit/event/enum.Ime.html>
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ImeEvent {
    /// Notifies when the IME was enabled.
    #[deprecated = "No longer used by egui"]
    Enabled,

    /// A new IME candidate is being suggested.
    ///
    /// An empty preedit string indicates that the IME has been dismissed, while
    /// a non-empty preedit string indicates that the IME is active.
    Preedit {
        text: String,
        active_range_chars: Option<std::ops::Range<usize>>,
        /// Диапазон (char-индексы) в существующем тексте, который этот preedit
        /// должен ЗАМЕНИТЬ перед вставкой `text`. Используется Android-интеграцией:
        /// Gboard шлёт `setComposingRegion(start, end)`, указывая, что композиция
        /// перестраивает уже введённый фрагмент (а не добавляется в конец). Если
        /// `None` — preedit вставляется в позицию курсора как обычно.
        replace_range: Option<std::ops::Range<usize>>,
    },

    /// IME composition ended with this final result.
    ///
    /// The IME is considered dismissed after this event.
    Commit(String),

    /// Notifies when the IME was disabled.
    #[deprecated = "No longer used by egui"]
    Disabled,
}
