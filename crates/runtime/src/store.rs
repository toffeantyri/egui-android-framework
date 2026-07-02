//! Реактивное состояние на основе `tokio::sync::watch`.
//!
//! Аналог `StateFlow` из Kotlin — observable state, который уведомляет
//! всех подписчиков при каждом изменении.
//!
//! # Пример
//!
//! ```ignore
//! let store = StateStore::new(0u32);
//! let rx = store.subscribe();
//!
//! // Где-то в reducer:
//! store.dispatch(Msg::Increment, |state, msg| match msg {
//!     Msg::Increment => *state += 1,
//! });
//!
//! // Подписчик увидит новое значение и может вызвать request_repaint()
//! std::thread::spawn(move || {
//!     while rx.has_changed().unwrap_or(false) {
//!         let _ = rx.borrow_and_update();
//!         ctx.request_repaint();
//!         std::thread::sleep(Duration::from_millis(5));
//!     }
//! });
//! ```

use tokio::sync::watch;

/// Реактивное состояние.
///
/// Единственный способ изменить состояние — через [`StateStore::update()`]
/// или [`StateStore::dispatch()`]. Прямая запись через `RwLock` запрещена.
///
/// Все подписчики (полученные через [`StateStore::subscribe()`]) получают
/// уведомление после каждого `update()`.
pub struct StateStore<T> {
    tx: watch::Sender<T>,
    rx: watch::Receiver<T>,
}

impl<T: Clone + Send + Sync + 'static> StateStore<T> {
    /// Создать новое хранилище с начальным значением.
    pub fn new(initial: T) -> Self {
        let (tx, rx) = watch::channel(initial);
        Self { tx, rx }
    }

    /// Атомарно изменить состояние через замыкание.
    ///
    /// После выполнения замыкания все подписчики получают уведомление
    /// (вызов `watch::Receiver::has_changed()` вернёт `true`).
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        self.tx.send_if_modified(|state| {
            f(state);
            true
        });
    }

    /// Отправить сообщение через Reducer.
    ///
    /// Reducer — чистая функция, которая получает текущее состояние
    /// и сообщение, и модифицирует состояние.
    ///
    /// Это единственная точка изменения состояния в MVI-архитектуре.
    pub fn dispatch<M>(&self, msg: M, reducer: fn(&mut T, M))
    where
        M: Send + 'static,
    {
        self.update(|state| reducer(state, msg));
    }

    /// Получить текущее состояние (snapshot).
    pub fn state(&self) -> T {
        self.rx.borrow().clone()
    }

    /// Подписаться на изменения состояния.
    pub fn subscribe(&self) -> watch::Receiver<T> {
        self.rx.clone()
    }

    /// Клонировать хранилище (создать новый handle с тем же каналом).
    pub fn clone_state(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            rx: self.rx.clone(),
        }
    }

    /// Внутренний отправитель — для продвинутого использования.
    /// Не рекомендуется вызывать напрямую вне фреймворка.
    #[doc(hidden)]
    pub fn sender(&self) -> watch::Sender<T> {
        self.tx.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_creation() {
        let store = StateStore::new(42u32);
        assert_eq!(store.state(), 42);
    }

    #[test]
    fn test_store_update() {
        let store = StateStore::new(0u32);
        store.update(|s| *s = 10);
        assert_eq!(store.state(), 10);
    }

    #[test]
    fn test_store_update_multiple() {
        let store = StateStore::new(0u32);
        store.update(|s| *s += 1);
        store.update(|s| *s += 2);
        store.update(|s| *s += 3);
        assert_eq!(store.state(), 6);
    }

    #[test]
    fn test_store_dispatch() {
        let store = StateStore::new(0u32);

        fn increment(state: &mut u32, _msg: ()) {
            *state += 1;
        }

        store.dispatch((), increment);
        store.dispatch((), increment);
        store.dispatch((), increment);

        assert_eq!(store.state(), 3);
    }

    #[test]
    fn test_store_dispatch_with_data() {
        let store = StateStore::new(String::new());

        fn append(state: &mut String, msg: &str) {
            state.push_str(msg);
        }

        store.dispatch("Hello, ", append);
        store.dispatch("World!", append);

        assert_eq!(store.state(), "Hello, World!");
    }

    #[test]
    fn test_store_subscribe_receives_updates() {
        let store = StateStore::new(0u32);
        let mut rx = store.subscribe();

        assert_eq!(*rx.borrow(), 0);

        store.update(|s| *s = 42);

        assert!(rx.has_changed().unwrap());
        let _ = rx.borrow_and_update();
        assert_eq!(*rx.borrow(), 42);
    }

    #[test]
    fn test_store_subscribe_multiple() {
        let store = StateStore::new(0u32);
        let mut rx1 = store.subscribe();
        let mut rx2 = store.subscribe();

        store.update(|s| *s = 99);

        assert!(rx1.has_changed().unwrap());
        assert!(rx2.has_changed().unwrap());
        let _ = rx1.borrow_and_update();
        let _ = rx2.borrow_and_update();
        assert_eq!(*rx1.borrow(), 99);
        assert_eq!(*rx2.borrow(), 99);
    }

    #[test]
    fn test_store_subscribe_no_duplicate_notifications() {
        let store = StateStore::new(0u32);
        let mut rx = store.subscribe();
        let mut count = 0;

        store.update(|s| *s = 1);
        if rx.has_changed().unwrap() {
            let _ = rx.borrow_and_update();
            count += 1;
        }

        store.update(|s| *s = 2);
        if rx.has_changed().unwrap() {
            let _ = rx.borrow_and_update();
            count += 1;
        }

        assert_eq!(count, 2);
        assert_eq!(*rx.borrow(), 2);
    }

    #[test]
    fn test_store_borrow_after_update() {
        let store = StateStore::new(vec![1, 2, 3]);
        store.update(|v| v.push(4));

        let state = store.state();
        assert_eq!(state.len(), 4);
        assert_eq!(state, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_store_update_with_struct() {
        #[derive(Clone, Debug, PartialEq)]
        struct AppState {
            count: u32,
            loading: bool,
            name: String,
        }

        let store = StateStore::new(AppState {
            count: 0,
            loading: true,
            name: String::new(),
        });

        store.update(|s| {
            s.loading = false;
            s.name = "Загружено".into();
        });

        let state = store.state();
        assert!(!state.loading);
        assert_eq!(state.name, "Загружено");
        assert_eq!(state.count, 0);
    }
}
