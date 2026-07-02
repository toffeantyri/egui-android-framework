//! Контекст компонента — [`ComponentContext`].
//!
//! Передаётся в компонент при создании и предоставляет:
//! - Навигационный handle (push/pop/replace родительского стека).
//! - Data layer handle (для отправки команд).
//! - Доступ к [`StateStore`] для реактивного получения состояния.
//!
//! Каналы реализованы через стандартный `std::sync::mpsc`.

use egui_android_runtime::StateStore;
use std::sync::mpsc;

/// Контекст, передаваемый в компонент при создании.
///
/// Содержит всё необходимое для взаимодействия с фреймворком,
/// родительским стеком и data layer.
///
/// # Параметры типа
///
/// * `NavEvent` — тип навигационного события (например, `NavEvent::Push(Screen::Home)`).
/// * `DataCmd` — тип команды для data layer.
/// * `State` — тип состояния приложения (хранится в `StateStore`).
pub struct ComponentContext<NavEvent, DataCmd, State>
where
    NavEvent: 'static,
    DataCmd: Send + 'static,
    State: Clone + Send + Sync + 'static,
{
    /// Отправитель навигационных событий в родительский стек.
    nav_tx: Option<mpsc::Sender<NavEvent>>,
    /// Отправитель команд в data layer.
    data_cmd_tx: mpsc::Sender<DataCmd>,
    /// Реактивное состояние приложения.
    store: StateStore<State>,
    /// Флаг: контекст жив (не уничтожен).
    alive: bool,
}

impl<NavEvent, DataCmd, State> ComponentContext<NavEvent, DataCmd, State>
where
    NavEvent: 'static,
    DataCmd: Send + 'static,
    State: Clone + Send + Sync + 'static,
{
    /// Создать новый контекст.
    ///
    /// Принимает опциональный навигационный Sender — `None` для root-компонента,
    /// у которого нет родительского стека.
    pub fn new(
        nav_tx: Option<mpsc::Sender<NavEvent>>,
        data_cmd_tx: mpsc::Sender<DataCmd>,
        store: StateStore<State>,
    ) -> Self {
        Self {
            nav_tx,
            data_cmd_tx,
            store,
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

    /// Получить доступ к реактивному состоянию приложения.
    ///
    /// Через `store.state()` можно получить snapshot,
    /// через `store.subscribe()` — подписаться на изменения.
    pub fn store(&self) -> &StateStore<State> {
        &self.store
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
/// разделяя общий `StateStore` и отправители.
pub struct ComponentContextHandle<NavEvent, DataCmd, State>
where
    NavEvent: 'static,
    DataCmd: Send + 'static,
    State: Clone + Send + Sync + 'static,
{
    nav_tx: mpsc::Sender<NavEvent>,
    data_cmd_tx: mpsc::Sender<DataCmd>,
    store: StateStore<State>,
}

impl<NavEvent, DataCmd, State> ComponentContextHandle<NavEvent, DataCmd, State>
where
    NavEvent: 'static,
    DataCmd: Send + 'static,
    State: Clone + Send + Sync + 'static,
{
    /// Создать новый handle.
    pub fn new(
        nav_tx: mpsc::Sender<NavEvent>,
        data_cmd_tx: mpsc::Sender<DataCmd>,
        store: StateStore<State>,
    ) -> Self {
        Self {
            nav_tx,
            data_cmd_tx,
            store,
        }
    }

    /// Создать новый ComponentContext для дочернего компонента.
    ///
    /// Дочерний компонент получает тот же навигационный Sender (отправит
    /// событие родительскому стеку), тот же data_cmd Sender и разделяемый
    /// StateStore.
    pub fn create_context(&self) -> ComponentContext<NavEvent, DataCmd, State> {
        ComponentContext::new(
            Some(self.nav_tx.clone()),
            self.data_cmd_tx.clone(),
            self.store.clone_state(),
        )
    }
}
