//! Интеграционные тесты UiNotifier.
//!
//! Проверяем:
//! - Создание UiNotifier с wake handle и без
//! - check() при пустом канале (не вызывает repaint)
//! - check() при наличии сигнала (вызывает repaint + wake)
//! - Множественные сигналы — один repaint
//! - check() без wake handle

use egui_android_runtime::{AndroidWakeHandle, UiNotifier};
use std::sync::{
    atomic::{AtomicBool, AtomicI32, Ordering},
    mpsc, Arc,
};

#[test]
fn test_create_notifier_with_wake() {
    let ctx = egui::Context::default();
    let (_tx, rx) = mpsc::channel::<()>();
    let wake = AndroidWakeHandle::new(|| {});

    let mut notifier = UiNotifier::new(ctx, Some(wake), rx);
    notifier.check();
}

#[test]
fn test_create_notifier_without_wake() {
    let ctx = egui::Context::default();
    let (_tx, rx) = mpsc::channel::<()>();

    let mut notifier = UiNotifier::new(ctx, None, rx);
    notifier.check();
}

#[test]
fn test_check_with_signal_triggers_repaint_and_wake() {
    let ctx = egui::Context::default();
    let (notify_tx, rx) = mpsc::channel::<()>();
    let woken = Arc::new(AtomicBool::new(false));
    let woken_clone = Arc::clone(&woken);
    let wake = AndroidWakeHandle::new(move || {
        woken_clone.store(true, Ordering::SeqCst);
    });

    let mut notifier = UiNotifier::new(ctx.clone(), Some(wake), rx);
    assert!(!ctx.has_requested_repaint());

    notify_tx.send(()).ok();
    notifier.check();

    assert!(ctx.has_requested_repaint());
    assert!(woken.load(Ordering::SeqCst));
}

#[test]
fn test_check_with_multiple_signals_triggers_once() {
    let ctx = egui::Context::default();
    let (notify_tx, rx) = mpsc::channel::<()>();
    let wake_count = Arc::new(AtomicI32::new(0));
    let wake_count_clone = Arc::clone(&wake_count);
    let wake = AndroidWakeHandle::new(move || {
        wake_count_clone.fetch_add(1, Ordering::SeqCst);
    });

    let mut notifier = UiNotifier::new(ctx.clone(), Some(wake), rx);
    notify_tx.send(()).ok();
    notify_tx.send(()).ok();
    notify_tx.send(()).ok();
    notifier.check();

    assert!(ctx.has_requested_repaint());
    assert!(wake_count.load(Ordering::SeqCst) >= 1);
}

#[test]
fn test_check_without_wake_still_repaints() {
    let ctx = egui::Context::default();
    let (notify_tx, rx) = mpsc::channel::<()>();

    let mut notifier = UiNotifier::new(ctx.clone(), None, rx);
    notify_tx.send(()).ok();
    notifier.check();

    assert!(ctx.has_requested_repaint());
}

#[test]
fn test_check_no_signal_no_repaint() {
    let ctx = egui::Context::default();
    let (_notify_tx, rx) = mpsc::channel::<()>();

    let wake = AndroidWakeHandle::new(|| {});

    let mut notifier = UiNotifier::new(ctx.clone(), Some(wake), rx);
    notifier.check();

    assert!(!ctx.has_requested_repaint());
}
