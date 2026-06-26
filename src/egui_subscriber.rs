//! Реактивный подписчик для egui + Android.
//!
//! Соединяет `watch::Receiver<T>` (от `StateStore`) с egui и Android
//! event loop. При каждом изменении состояния:
//!
//! 1. Вызывает `egui::Context::request_repaint()`
//! 2. Вызывает `AndroidWakeHandle::wake()` (пробуждает главный цикл)
//!
//! Это устраняет необходимость в ручном `poll()` — UI обновляется
//! автоматически после любого изменения состояния.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::watch;

/// Подписчик на изменения состояния.
///
/// Запускает фоновый поток, который ждёт изменений `watch::Receiver`
/// и при каждом изменении вызывает `request_repaint()` и `wake()`.
///
/// Автоматически завершает поток при `Drop`.
///
/// # Пример
///
/// ```ignore
/// let store = StateStore::new(my_state);
/// let rx = store.subscribe();
/// let _subscriber = EguiRepaintSubscriber::new(rx, egui_ctx, wake_handle);
/// ```
pub struct EguiRepaintSubscriber {
    /// Флаг остановки потока.
    stop_flag: Arc<AtomicBool>,
    /// JoinHandle фонового потока (опционально, для join при тестах).
    _handle: Option<std::thread::JoinHandle<()>>,
}

impl EguiRepaintSubscriber {
    /// Создать подписчика.
    ///
    /// # Параметры
    ///
    /// * `rx` — `watch::Receiver` от `StateStore::subscribe()`.
    /// * `ctx` — egui контекст для `request_repaint()`.
    /// * `wake` — опциональный waker для Android event loop.
    ///
    /// Фоновый поток живёт, пока `EguiRepaintSubscriber` не будет
    /// уничтожен (RAII).
    pub fn new<T: Send + Sync + 'static>(
        mut rx: watch::Receiver<T>,
        ctx: egui::Context,
        wake: Option<AndroidWakeHandle>,
    ) -> Self {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop = Arc::clone(&stop_flag);

        let handle = std::thread::Builder::new()
            .name("egui-subscriber".into())
            .spawn(move || {
                // Синхронный polling-цикл через has_changed().
                // tokio::sync::watch не имеет blocking_changed(), поэтому
                // используем sleeps с проверкой has_changed().
                //
                // Это не идеально с точки зрения latency, но для UI-фреймворка
                // с целевым FPS 30-60 задержка 1-5ms не критична.
                loop {
                    // Проверяем флаг остановки
                    if stop.load(Ordering::Relaxed) {
                        log::debug!("EguiSubscriber: поток завершён");
                        break;
                    }

                    // Проверяем, появилось ли новое значение
                    match rx.has_changed() {
                        Ok(true) => {
                            log::info!("EguiSubscriber: изменение обнаружено, вызываем request_repaint + wake");
                            // Помечаем как прочитанное, чтобы has_changed()
                            // снова вернул true только при следующем send()
                            let _ = rx.borrow_and_update();

                            // 1. Запрашиваем repaint у egui
                            ctx.request_repaint();

                            // 2. Будим Android event loop (если есть waker)
                            if let Some(ref w) = wake {
                                w.wake();
                            }
                        }
                        Ok(false) => {
                            // Нет изменений — спим немного
                            std::thread::sleep(std::time::Duration::from_millis(5));
                        }
                        Err(_) => {
                            // Канал закрыт — все отправители уничтожены.
                            log::debug!("EguiSubscriber: канал закрыт, завершаемся");
                            break;
                        }
                    }
                }
            });

        match handle {
            Ok(h) => Self {
                stop_flag,
                _handle: Some(h),
            },
            Err(e) => {
                log::error!("EguiSubscriber: не удалось запустить поток: {e}");
                Self {
                    stop_flag,
                    _handle: None,
                }
            }
        }
    }
}

impl Drop for EguiRepaintSubscriber {
    fn drop(&mut self) {
        // Сигнализируем потоку остановиться
        self.stop_flag.store(true, Ordering::Relaxed);

        // Если есть handle — ждём завершения с таймаутом
        if let Some(handle) = self._handle.take() {
            let _ = handle.join();
        }
    }
}

/// Handle для пробуждения Android event loop.
///
/// Создаётся из замыкания, которое вызывает `AndroidApp::wake()`.
/// В тестах можно использовать заглушку.
///
/// # Пример
///
/// ```ignore
/// let wake = AndroidWakeHandle::new(move || { let _ = app.wake(); });
/// ```
pub struct AndroidWakeHandle {
    wake_fn: Arc<dyn Fn() + Send + Sync>,
}

impl Clone for AndroidWakeHandle {
    fn clone(&self) -> Self {
        Self {
            wake_fn: Arc::clone(&self.wake_fn),
        }
    }
}

impl AndroidWakeHandle {
    /// Создать новый handle.
    pub fn new(wake_fn: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            wake_fn: Arc::new(wake_fn),
        }
    }

    /// Пробудить Android event loop.
    ///
    /// Вызывает `MainEvent::Wake` в главном цикле `run()`.
    pub fn wake(&self) {
        (self.wake_fn)();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::StateStore;

    #[test]
    fn test_wake_handle() {
        let woke = Arc::new(AtomicBool::new(false));
        let woke_clone = Arc::clone(&woke);

        let wake = AndroidWakeHandle::new(move || {
            woke_clone.store(true, Ordering::SeqCst);
        });

        assert!(!woke.load(Ordering::SeqCst));
        wake.wake();
        assert!(woke.load(Ordering::SeqCst));
    }

    #[test]
    fn test_subscriber_triggers_repaint_on_update() {
        let store = StateStore::new(0u32);
        let rx = store.subscribe();
        let ctx = egui::Context::default();

        // Создаём подписчика (без wake, только request_repaint)
        let _subscriber = EguiRepaintSubscriber::new(rx, ctx.clone(), None);

        // Даём потоку время стартовать
        std::thread::sleep(std::time::Duration::from_millis(50));

        // После update() подписчик должен вызвать request_repaint()
        store.update(|s| *s = 1);

        // Даём время обработать (polling с 5ms sleep)
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Проверяем, что ctx помнит о repaint
        assert!(ctx.has_requested_repaint());
    }

    #[test]
    fn test_subscriber_drop_stops_thread() {
        let store = StateStore::new(0u32);
        let rx = store.subscribe();
        let ctx = egui::Context::default();

        let stop_flag: Arc<AtomicBool>;
        {
            let subscriber = EguiRepaintSubscriber::new(rx, ctx, None);
            stop_flag = Arc::clone(&subscriber.stop_flag);
            // subscriber уничтожается здесь
        }

        // После Drop поток должен завершиться
        assert!(stop_flag.load(Ordering::Relaxed));
    }

    #[test]
    fn test_subscriber_multiple_updates() {
        let store = StateStore::new(0u32);
        let rx = store.subscribe();
        let ctx = egui::Context::default();

        let _subscriber = EguiRepaintSubscriber::new(rx, ctx.clone(), None);

        std::thread::sleep(std::time::Duration::from_millis(50));

        // Множественные обновления
        for i in 1..=5 {
            store.update(|s| *s = i);
            std::thread::sleep(std::time::Duration::from_millis(30));
            assert!(
                ctx.has_requested_repaint(),
                "repaint должен быть на update {i}"
            );
            // Сбросить флаг repaint нечем — просто проверяем, что он стал true
        }
    }

    #[test]
    fn test_subscriber_wake_called() {
        let store = StateStore::new(0u32);
        let rx = store.subscribe();
        let ctx = egui::Context::default();
        let woke = Arc::new(AtomicBool::new(false));
        let w = Arc::clone(&woke);

        let wake = AndroidWakeHandle::new(move || {
            w.store(true, Ordering::SeqCst);
        });

        let _subscriber = EguiRepaintSubscriber::new(rx, ctx, Some(wake));

        std::thread::sleep(std::time::Duration::from_millis(50));

        store.update(|s| *s = 42);

        std::thread::sleep(std::time::Duration::from_millis(100));

        assert!(
            woke.load(Ordering::SeqCst),
            "wake() должен быть вызван при update"
        );
    }
}
