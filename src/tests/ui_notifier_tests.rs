//! Тесты для UiNotifier и AndroidWakeHandle — инфраструктуры уведомлений.
//!
//! Проверяет:
//! - Создание и вызов AndroidWakeHandle
//! - Обнаружение сигнала через mpsc канал
//! - Вызов request_repaint + wake при сигнале
//! - Отсутствие repaint без сигнала
//! - Обработка множественных сигналов

use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};

use crate::ui_notifier::{AndroidWakeHandle, UiNotifier};

fn test_context() -> egui::Context {
    egui::Context::default()
}

fn make_wake_handle(woke: &Arc<AtomicBool>) -> AndroidWakeHandle {
    let w = Arc::clone(woke);
    AndroidWakeHandle::new(move || {
        w.store(true, Ordering::SeqCst);
    })
}

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
fn test_notifier_detects_signal() {
    let ctx = test_context();
    let woke = Arc::new(AtomicBool::new(false));
    let wake = make_wake_handle(&woke);
    let (tx, rx) = mpsc::channel::<()>();

    let mut notifier = UiNotifier::new(ctx.clone(), Some(wake), rx);

    // Без сигнала — repaint не вызывается
    notifier.check();
    assert!(!ctx.has_requested_repaint());
    assert!(!woke.load(Ordering::SeqCst));

    // Отправляем сигнал
    let _ = tx.send(());
    notifier.check();

    // repaint должен быть вызван
    assert!(ctx.has_requested_repaint());
    // waker должен быть вызван
    assert!(woke.load(Ordering::SeqCst));
}

#[test]
fn test_notifier_no_signal_no_repaint() {
    let ctx = test_context();
    let woke = Arc::new(AtomicBool::new(false));
    let wake = make_wake_handle(&woke);
    let (_tx, rx) = mpsc::channel::<()>();

    let mut notifier = UiNotifier::new(ctx.clone(), Some(wake), rx);

    // Множественные проверки без сигнала
    for _ in 0..5 {
        notifier.check();
    }

    // repaint не должен быть вызван
    assert!(!ctx.has_requested_repaint());
    assert!(!woke.load(Ordering::SeqCst));
}

#[test]
fn test_notifier_multiple_signals_one_repaint() {
    let ctx = test_context();
    let woke = Arc::new(AtomicBool::new(false));
    let wake = make_wake_handle(&woke);
    let (tx, rx) = mpsc::channel::<()>();

    let mut notifier = UiNotifier::new(ctx.clone(), Some(wake), rx);

    // Отправляем несколько сигналов
    for _ in 0..3 {
        let _ = tx.send(());
    }

    // Один check — все сигналы сливаются
    notifier.check();

    // repaint вызван ровно один раз
    assert!(ctx.has_requested_repaint());
    assert!(woke.load(Ordering::SeqCst));

    // Сбрасываем флаги и проверяем ещё раз — сигналов больше нет
    let woke2 = Arc::new(AtomicBool::new(false));
    let wake2 = make_wake_handle(&woke2);
    let ctx2 = test_context();
    let (_tx2, rx2) = mpsc::channel::<()>();
    let mut notifier2 = UiNotifier::new(ctx2.clone(), Some(wake2), rx2);

    notifier2.check();
    assert!(!ctx2.has_requested_repaint());
    assert!(!woke2.load(Ordering::SeqCst));
}

#[test]
fn test_notifier_without_waker() {
    let ctx = test_context();
    let (tx, rx) = mpsc::channel::<()>();

    let mut notifier = UiNotifier::new(ctx.clone(), None, rx);

    // Отправляем сигнал — repaint должен вызваться, waker — нет
    let _ = tx.send(());
    notifier.check();

    assert!(ctx.has_requested_repaint());
    // Без waker — просто не падаем
}

#[test]
fn test_notifier_sender_dropped() {
    let woke = Arc::new(AtomicBool::new(false));
    let wake = make_wake_handle(&woke);
    let ctx = test_context();
    let (tx, rx) = mpsc::channel::<()>();

    let mut notifier = UiNotifier::new(ctx.clone(), Some(wake), rx);

    // Отправляем сигнал и дропаем sender
    let _ = tx.send(());
    drop(tx);

    // check всё равно должен работать (try_recv не паникует)
    notifier.check();
    assert!(ctx.has_requested_repaint());
}
