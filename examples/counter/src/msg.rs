//! Типы сообщений для примера счётчика.

/// Состояние счётчика (хранится в StateStore).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct CounterState {
    pub count: u32,
}

/// Сообщение от View к компоненту.
#[derive(Debug, Clone)]
pub enum Msg {
    Increment,
    /// Переключить тему (светлая/тёмная).
    ToggleTheme,
}
