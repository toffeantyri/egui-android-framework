//! Типы сообщений и состояния для примера счётчика.

/// Состояние счётчика (хранится в StateStore).
#[derive(Clone, Debug, PartialEq)]
pub struct CounterState {
    pub count: u32,
}

/// Сообщение от View к компоненту.
#[derive(Debug, Clone)]
pub enum Msg {
    Increment,
}

/// Событие от data layer.
///
/// **Устарел.** Используйте `StateStore<CounterState>` вместо mpsc-канала.
/// События публикуются через `store.update()` и приходят реактивно.
#[derive(Debug)]
pub enum Evt {
    CountUpdated(u32),
}
