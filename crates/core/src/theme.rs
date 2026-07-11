//! Системная тема (Light/Dark) — общий тип для крейтов platform-android и ui.

/// Системная тема Android: светлая или тёмная.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemTheme {
    /// Светлая тема.
    Light,
    /// Тёмная тема.
    Dark,
}
