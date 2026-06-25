//! Типы сообщений для примера счётчика.

/// Сообщение от View к компоненту.
#[derive(Debug, Clone)]
pub enum Msg {
    Increment,
}

/// Событие от data layer.
#[derive(Debug)]
pub enum Evt {
    CountUpdated(u32),
}
