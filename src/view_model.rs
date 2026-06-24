use std::sync::{mpsc, Arc, Mutex};

pub trait ViewModel: Sized + Send {
    type DataCommand: Send + 'static;
    type Event: Send + 'static;

    fn create(context: ViewModelContext<Self::DataCommand, Self::Event>) -> Self;

    /// Обработать команду от UI.
    ///
    /// Вызывается через [`dispatch()`]. Аналог редьюсера — изменяет состояние
    /// ViewModel в ответ на команду пользователя.
    fn handle(&mut self, cmd: Self::DataCommand);

    /// Отправить команду в ViewModel на обработку.
    ///
    /// Вызывается из Activity. По умолчанию делегирует [`handle()`],
    /// но может быть переопределён для логирования, отладки или
    /// перехвата команд до обработки.
    fn dispatch(&mut self, cmd: Self::DataCommand) {
        self.handle(cmd);
    }

    /// Реагировать на событие из data layer.
    ///
    /// Вызывается фреймворком после `poll_events()`, перед `render()`.
    fn on_event(&mut self, _evt: Self::Event) {}
}

pub struct ViewModelContext<C: Send + 'static, E: Send + 'static> {
    command_tx: mpsc::Sender<C>,
    event_rx: Arc<Mutex<mpsc::Receiver<E>>>,
}

impl<C: Send + 'static, E: Send + 'static> ViewModelContext<C, E> {
    pub fn new(tx: mpsc::Sender<C>, rx: Arc<Mutex<mpsc::Receiver<E>>>) -> Self {
        Self {
            command_tx: tx,
            event_rx: rx,
        }
    }

    /// Создать контекст из готового Sender и общего Arc<Mutex<Receiver>>.
    /// Используется для клонирования контекста (run() получает свою копию).
    pub fn from_parts(tx: mpsc::Sender<C>, rx: Arc<Mutex<mpsc::Receiver<E>>>) -> Self {
        Self {
            command_tx: tx,
            event_rx: rx,
        }
    }

    /// Get a reference to the command sender.
    ///
    /// Useful when the caller needs to clone the sender for use outside
    /// of the ViewModel (e.g. from the UI thread during `render()`).
    pub fn command_tx(&self) -> &mpsc::Sender<C> {
        &self.command_tx
    }

    /// Отправить команду в data layer.
    ///
    /// ViewModel вызывает этот метод, чтобы передать команду
    /// во внешний слой (сервис, репозиторий).
    pub fn send(&self, cmd: C) {
        let _ = self.command_tx.send(cmd);
    }

    /// Забрать накопившиеся события из data layer.
    ///
    /// Вызывается фреймворком перед каждым `render()`,
    /// результат передаётся в `ViewModel::on_event()`.
    pub fn poll_events(&self) -> Vec<E> {
        let rx = self.event_rx.lock().unwrap();
        let mut events = Vec::new();
        while let Ok(evt) = rx.try_recv() {
            events.push(evt);
        }
        events
    }
}
