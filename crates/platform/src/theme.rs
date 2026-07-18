//! Системная тема (Light/Dark) — общий тип для платформенного слоя.

/// Системная тема: светлая или тёмная.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemTheme {
    /// Светлая тема.
    Light,
    /// Тёмная тема.
    Dark,
}
