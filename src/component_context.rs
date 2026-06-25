//! Контекст компонента — [`ComponentContext`].
//!
//! Передаётся в компонент при создании и предоставляет:
//! - Навигационный handle (push/pop/replace родительского стека).
//! - Data layer handle (для отправки команд и получения событий).
//!
//! Каналы реализованы через стандартный `std::sync::mpsc`,
//! Sender'ы клонируются для раздачи дочерним компонентам.

use std::sync::{mpsc, Arc, Mutex};

/// Контекст, передаваемый в компонент при создании.
///
/// Содержит всё необходимое для взаимодействия с фреймворком,
/// родительским стеком и data layer.
///
/// # Параметры типа
///
/// * `NavEvent` — тип навигационного события (например, `NavEvent::Push(Screen::Home)`).
/// * `DataCmd` — тип команды для data layer.
/// * `DataEvt` — тип события от data layer.
pub struct ComponentContext<NavEvent, DataCmd, DataEvt>
where
    NavEvent: 'static,
    DataCmd: Send + 'static,
    DataEvt: Send + 'static,
{
    /// Отправитель навигационных событий в родительский стек.
    nav_tx: Option<mpsc::Sender<NavEvent>>,
    /// Отправитель команд в data layer.
    data_cmd_tx: mpsc::Sender<DataCmd>,
    /// Получатель событий от data layer (разделённый через Arc).
    data_evt_rx: Arc<Mutex<mpsc::Receiver<DataEvt>>>,
    /// Флаг: контекст жив (не уничтожен).
    alive: bool,
}

impl<NavEvent, DataCmd, DataEvt> ComponentContext<NavEvent, DataCmd, DataEvt>
where
    NavEvent: 'static,
    DataCmd: Send + 'static,
    DataEvt: Send + 'static,
{
    /// Создать новый контекст.
    ///
    /// Принимает опциональный навигационный Sender — `None` для root-компонента,
    /// у которого нет родительского стека.
    pub fn new(
        nav_tx: Option<mpsc::Sender<NavEvent>>,
        data_cmd_tx: mpsc::Sender<DataCmd>,
        data_evt_rx: Arc<Mutex<mpsc::Receiver<DataEvt>>>,
    ) -> Self {
        Self {
            nav_tx,
            data_cmd_tx,
            data_evt_rx,
            alive: true,
        }
    }

    /// Отправить навигационное событие родителю (push/pop/replace).
    ///
    /// Безопасно игнорирует вызов, если контекст создан без навигационного канала
    /// (например, для root-компонента).
    pub fn send_nav(&self, event: NavEvent) {
        if let Some(ref nav_tx) = self.nav_tx {
            let _ = nav_tx.send(event);
        }
    }

    /// Отправить команду в data layer.
    pub fn send_cmd(&self, cmd: DataCmd) {
        let _ = self.data_cmd_tx.send(cmd);
    }

    /// Забрать накопившиеся события из data layer.
    pub fn poll_events(&self) -> Vec<DataEvt> {
        let rx = self.data_evt_rx.lock().unwrap();
        let mut events = Vec::new();
        while let Ok(evt) = rx.try_recv() {
            events.push(evt);
        }
        events
    }

    /// Получить отправитель команд (для клонирования).
    pub fn data_cmd_tx(&self) -> mpsc::Sender<DataCmd> {
        self.data_cmd_tx.clone()
    }

    /// Получить отправитель навигации (для клонирования).
    ///
    /// Возвращает `None`, если навигационный канал не был настроен
    /// (root-компонент).
    pub fn nav_tx(&self) -> Option<mpsc::Sender<NavEvent>> {
        self.nav_tx.clone()
    }

    /// Отметить контекст как уничтоженный.
    pub fn mark_destroyed(&mut self) {
        self.alive = false;
    }

    /// Проверить, жив ли контекст (не уничтожен).
    pub fn is_alive(&self) -> bool {
        self.alive
    }
}

/// Хранилище для ComponentContext.
///
/// Позволяет создавать новые контексты для дочерних компонентов,
/// разделяя общий `Arc<Mutex<Receiver<DataEvt>>>`.
pub struct ComponentContextHandle<NavEvent, DataCmd, DataEvt>
where
    NavEvent: 'static,
    DataCmd: Send + 'static,
    DataEvt: Send + 'static,
{
    nav_tx: mpsc::Sender<NavEvent>,
    data_cmd_tx: mpsc::Sender<DataCmd>,
    data_evt_rx: Arc<Mutex<mpsc::Receiver<DataEvt>>>,
}

impl<NavEvent, DataCmd, DataEvt> ComponentContextHandle<NavEvent, DataCmd, DataEvt>
where
    NavEvent: 'static,
    DataCmd: Send + 'static,
    DataEvt: Send + 'static,
{
    /// Создать новый handle.
    pub fn new(
        nav_tx: mpsc::Sender<NavEvent>,
        data_cmd_tx: mpsc::Sender<DataCmd>,
        data_evt_rx: Arc<Mutex<mpsc::Receiver<DataEvt>>>,
    ) -> Self {
        Self {
            nav_tx,
            data_cmd_tx,
            data_evt_rx,
        }
    }

    /// Создать новый ComponentContext для дочернего компонента.
    ///
    /// Дочерний компонент получает тот же навигационный Sender (отправит
    /// событие родительскому стеку), тот же data_cmd Sender и разделяемый
    /// Receiver событий от data layer.
    pub fn create_context(&self) -> ComponentContext<NavEvent, DataCmd, DataEvt> {
        ComponentContext::new(
            Some(self.nav_tx.clone()),
            self.data_cmd_tx.clone(),
            Arc::clone(&self.data_evt_rx),
        )
    }
}
