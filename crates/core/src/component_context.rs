//! Контекст компонента — [`ComponentContext`].
//!
//! Передаётся в компонент при создании и предоставляет:
//! - BackDispatcher для регистрации обработчиков Back.
//! - Fallback-callback (on_back_fallback) для pop из ChildStack или завершения.
//! - Data layer handle (для отправки команд).
//! - Доступ к [`StateStore`] для реактивного получения состояния.
//! - `saved_state` — сохранённое состояние для восстановления после пересоздания.
//!
//! Каналы реализованы через стандартный `std::sync::mpsc`.
//!
//! # Обработка Back
//!
//! `ComponentContext::on_back()` — единая точка входа для Back:
//! 1. Вызывает `back_dispatcher.handle()` (диалоги, nested stacks).
//! 2. Если никто не обработал → вызывает `back_fallback` (pop/finish).
//! 3. Если нет fallback → возвращает `false`.
//!
//! Системный Back (из platform-android) и UI Back (кнопка "← Назад")
//! идут через один и тот же путь — `on_back()`.
//!
//! # Сохранение состояния
//!
//! `saved_state` хранит `Option<Box<dyn Any + Send>>` — произвольное состояние,
//! которое компонент может сохранить перед уничтожением (через `ComponentNode::save_state()`)
//! и восстановить после пересоздания (через `ComponentNode::restore_state()`).
//! Используется платформой при Lifecycle::Destroy / InitWindow.

use crate::back_dispatcher::BackDispatcher;
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
    navevent_tx: Option<mpsc::Sender<NavEvent>>,
    /// Отправитель команд в data layer.
    datacmd_tx: mpsc::Sender<DataCmd>,
    /// Реактивное состояние приложения.
    store: StateStore<State>,
    /// Флаг: контекст жив (не уничтожен).
    alive: bool,
    /// BackDispatcher для регистрации обработчиков Back.
    pub back_dispatcher: BackDispatcher,
    /// Fallback при Back, когда BackDispatcher не обработал.
    ///
    /// Устанавливается владельцем ChildStack (RootComponent, NestedScreen).
    /// Вызывается из `on_back()` если `back_dispatcher.handle()` вернул `false`.
    /// Возвращает `true` если Back обработан (pop сделан), `false` если нет.
    ///
    /// Должен быть `Send` так как `ComponentContext` используется в `Component: Send`.
    pub back_fallback: Option<Box<dyn FnMut() -> bool + Send>>,
    /// Флаг: запрошено завершение приложения (стек навигации пуст).
    ///
    /// Устанавливается `back_fallback-ом` когда ChildStack пуст.
    /// Читается `Application::request_destroy()`.
    pub finish_requested: bool,
    /// Сохранённое состояние для восстановления после пересоздания.
    ///
    /// Устанавливается из `ComponentNode::save_state()` при уничтожении,
    /// передаётся в `restore_state()` при создании.
    pub saved_state: Option<Box<dyn std::any::Any + Send>>,
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
        navevent_tx: Option<mpsc::Sender<NavEvent>>,
        datacmd_tx: mpsc::Sender<DataCmd>,
        store: StateStore<State>,
    ) -> Self {
        Self {
            navevent_tx,
            datacmd_tx,
            store,
            alive: true,
            back_dispatcher: BackDispatcher::new(),
            back_fallback: None,
            finish_requested: false,
            saved_state: None,
        }
    }

    /// Обработать Back — единая точка входа.
    ///
    /// 1. Вызывает `back_dispatcher.handle()` — зарегистрированные callback'и
    ///    (диалоги, nested stacks) от высокого приоритета к низшему.
    /// 2. Если никто не обработал → вызывает `back_fallback` (pop/finish).
    /// 3. Если нет fallback → возвращает `false`.
    ///
    /// Возвращает `true`, если Back обработан.
    pub fn on_back(&mut self) -> bool {
        if self.back_dispatcher.handle() {
            return true;
        }

        if let Some(ref mut fallback) = self.back_fallback {
            return fallback();
        }

        false
    }

    /// Отправить навигационное событие родителю (push/pop/replace).
    ///
    /// Безопасно игнорирует вызов, если контекст создан без навигационного канала
    /// (например, для root-компонента).
    pub fn send_nav(&self, event: NavEvent) {
        if let Some(ref navevent_tx) = self.navevent_tx {
            let _ = navevent_tx.send(event);
        }
    }

    /// Отправить команду в data layer.
    pub fn send_cmd(&self, cmd: DataCmd) {
        let _ = self.datacmd_tx.send(cmd);
    }

    /// Получить доступ к реактивному состоянию приложения.
    ///
    /// Через `store.state()` можно получить snapshot,
    /// через `store.subscribe()` — подписаться на изменения.
    pub fn store(&self) -> &StateStore<State> {
        &self.store
    }

    /// Получить отправитель команд (для клонирования).
    pub fn datacmd_tx(&self) -> mpsc::Sender<DataCmd> {
        self.datacmd_tx.clone()
    }

    /// Получить отправитель навигации (для клонирования).
    ///
    /// Возвращает `None`, если навигационный канал не был настроен
    /// (root-компонент).
    pub fn navevent_tx(&self) -> Option<mpsc::Sender<NavEvent>> {
        self.navevent_tx.clone()
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
    navevent_tx: mpsc::Sender<NavEvent>,
    datacmd_tx: mpsc::Sender<DataCmd>,
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
        navevent_tx: mpsc::Sender<NavEvent>,
        datacmd_tx: mpsc::Sender<DataCmd>,
        store: StateStore<State>,
    ) -> Self {
        Self {
            navevent_tx,
            datacmd_tx,
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
            Some(self.navevent_tx.clone()),
            self.datacmd_tx.clone(),
            self.store.clone_state(),
        )
    }
}
