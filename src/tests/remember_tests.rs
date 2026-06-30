//! Тесты для локального состояния (remember, RememberState).
//!
//! Проверяет:
//! - remember инициализирует значение при первом вызове
//! - remember сохраняет значение между кадрами
//! - remember.modify корректно изменяет значение
//! - Разные ключи — разное состояние
//! - set перезаписывает значение
//! - init вызывается лениво (только при первом обращении)

use std::cell::RefCell;

use crate::{remember, RememberState};

fn with_ui(f: impl FnOnce(&mut egui::Ui)) {
    let f = RefCell::new(Some(f));
    let ctx = egui::Context::default();
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let f = f.borrow_mut().take().unwrap();
            f(ui);
        });
    });
}

// ─── remember ────────────────────────────────────────────────────────────────

#[test]
fn remember_initializes_value_on_first_call() {
    with_ui(|ui| {
        let state = remember(ui, "test_key", || 42);
        assert_eq!(*state.get(), 42);
    });
}

#[test]
fn remember_preserves_value_between_frames() {
    let ctx = egui::Context::default();

    // Первый кадр — инициализация и установка
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut state = remember(ui, "test_key", || 0);
            state.set(100);
        });
    });

    // Второй кадр — значение сохранено
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let state = remember(ui, "test_key", || 0);
            assert_eq!(*state.get(), 100);
        });
    });
}

#[test]
fn remember_modify_updates_value() {
    with_ui(|ui| {
        let mut state = remember(ui, "test_key", || 10);
        state.modify(|v| *v += 5);
        assert_eq!(*state.get(), 15);
    });
}

#[test]
fn different_keys_have_different_state() {
    with_ui(|ui| {
        let state1 = remember(ui, "key1", || 1);
        let state2 = remember(ui, "key2", || 2);
        assert_eq!(*state1.get(), 1);
        assert_eq!(*state2.get(), 2);
    });
}

#[test]
fn remember_set_overwrites_value() {
    with_ui(|ui| {
        let mut state = remember(ui, "test_key", || 0);
        state.set(42);
        assert_eq!(*state.get(), 42);
        state.set(99);
        assert_eq!(*state.get(), 99);
    });
}

#[test]
fn remember_init_called_lazily() {
    let ctx = egui::Context::default();
    let mut call_count = 0;

    // Первый кадр — init срабатывает
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let state = remember(ui, "lazy_key", || {
                call_count += 1;
                42
            });
            assert_eq!(*state.get(), 42);
            assert_eq!(call_count, 1);
        });
    });

    // Второй кадр на том же ctx — init НЕ вызывается
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let state = remember(ui, "lazy_key", || {
                call_count += 1;
                0
            });
            assert_eq!(*state.get(), 42);
            assert_eq!(call_count, 1, "init должен вызваться только один раз");
        });
    });
}

// ─── RememberState<String> ───────────────────────────────────────────────────

#[test]
fn remember_works_with_string() {
    with_ui(|ui| {
        let mut state = remember(ui, "str_key", || String::from("hello"));
        assert_eq!(state.get(), "hello");
        state.set(String::from("world"));
        assert_eq!(state.get(), "world");
    });
}

#[test]
fn remember_works_with_bool() {
    with_ui(|ui| {
        let mut state = remember(ui, "bool_key", || true);
        assert!(*state.get());
        state.modify(|v| *v = false);
        assert!(!*state.get());
    });
}

#[test]
fn remember_works_with_complex_struct() {
    #[derive(Clone, Debug, PartialEq)]
    struct ComplexState {
        name: String,
        count: u32,
        active: bool,
    }

    with_ui(|ui| {
        let mut state = remember(ui, "complex_key", || ComplexState {
            name: "test".into(),
            count: 0,
            active: false,
        });
        assert_eq!(state.get().name, "test");
        state.modify(|s| {
            s.count += 1;
            s.active = true;
        });
        assert_eq!(state.get().count, 1);
        assert!(state.get().active);
    });
}

// ─── RememberState со сложными ключами ───────────────────────────────────────

#[test]
fn remember_accepts_tuple_as_key() {
    with_ui(|ui| {
        let state = remember(ui, ("section", 1, "field"), || 42);
        assert_eq!(*state.get(), 42);
    });
}

// ─── Тип RememberState без вызова remember (для статических сценариев) ───────

#[test]
fn remember_returns_same_type_across_calls() {
    with_ui(|ui| {
        let state: RememberState<i32> = remember(ui, "typed_key", || 0);
        assert_eq!(*state.get(), 0);
    });
}
