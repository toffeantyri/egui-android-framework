use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;

use egui_android_platform::Waker;
use egui_android_runtime::ui_notifier::UiNotifier;

#[test]
fn test_create_notifier() {
    let ctx = egui::Context::default();
    let (_statechanged_tx, statechanged_rx) = mpsc::channel::<()>();
    let _wake = Waker::new(|| {});

    let mut notifier = UiNotifier::new(ctx, None, statechanged_rx);
    notifier.check();
}

#[test]
fn test_create_notifier_with_wake() {
    let ctx = egui::Context::default();
    let (_statechanged_tx, statechanged_rx) = mpsc::channel::<()>();
    let wake = Waker::new(|| {});

    let mut notifier = UiNotifier::new(ctx, Some(wake), statechanged_rx);
    notifier.check();
}

#[test]
fn test_create_notifier_without_wake() {
    let ctx = egui::Context::default();
    let (_statechanged_tx, statechanged_rx) = mpsc::channel::<()>();

    let mut notifier = UiNotifier::new(ctx, None, statechanged_rx);
    notifier.check();
}

#[test]
fn test_check_with_signal_triggers_repaint_and_wake() {
    let ctx = egui::Context::default();
    let (statechanged_tx, statechanged_rx) = mpsc::channel::<()>();
    let woken = Arc::new(AtomicBool::new(false));
    let woken_clone = Arc::clone(&woken);
    let wake = Waker::new(move || {
        woken_clone.store(true, Ordering::SeqCst);
    });

    let mut notifier = UiNotifier::new(ctx.clone(), Some(wake), statechanged_rx);

    statechanged_tx.send(()).ok();
    notifier.check();

    assert!(woken.load(Ordering::SeqCst));
}

#[test]
fn test_check_no_signal_no_repaint() {
    let ctx = egui::Context::default();
    let (_statechanged_tx, statechanged_rx) = mpsc::channel::<()>();
    let wake = Waker::new(|| {});

    let mut notifier = UiNotifier::new(ctx.clone(), Some(wake), statechanged_rx);
    notifier.check();
}

#[test]
fn test_check_with_signal_clears_repaint_request() {
    let ctx = egui::Context::default();
    let (statechanged_tx, statechanged_rx) = mpsc::channel::<()>();
    let wake = Waker::new(|| {});

    let mut notifier = UiNotifier::new(ctx.clone(), Some(wake), statechanged_rx);

    statechanged_tx.send(()).ok();
    notifier.check();
}
