use egui_android_ui::{remember, RememberState};
use std::cell::RefCell;

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
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let state = remember(ui, "test_key", || 0);
            state.set(100);
        });
    });
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
        let state = remember(ui, "test_key", || 10);
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
        let state = remember(ui, "test_key", || 0);
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
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let state = remember(ui, "lazy_key", || {
                call_count += 1;
                0
            });
            assert_eq!(*state.get(), 42);
            assert_eq!(call_count, 1, "init должен вызываться только один раз");
        });
    });
}

#[test]
fn remember_works_with_string() {
    with_ui(|ui| {
        let state = remember(ui, "str_key", || String::from("hello"));
        assert_eq!(&*state.get(), "hello");
        state.set(String::from("world"));
        assert_eq!(&*state.get(), "world");
    });
}

#[test]
fn remember_works_with_bool() {
    with_ui(|ui| {
        let state = remember(ui, "bool_key", || true);
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
        let state = remember(ui, "complex_key", || ComplexState {
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

#[test]
fn remember_accepts_tuple_as_key() {
    with_ui(|ui| {
        let state = remember(ui, ("section", 1, "field"), || 42);
        assert_eq!(*state.get(), 42);
    });
}

#[test]
fn remember_returns_same_type_across_calls() {
    with_ui(|ui| {
        let state: RememberState<i32> = remember(ui, "typed_key", || 0);
        assert_eq!(*state.get(), 0);
    });
}

#[test]
fn remember_set_get_same_frame() {
    with_ui(|ui| {
        let state = remember(ui, "frame_key", || 0i32);
        assert_eq!(*state.get(), 0);
        state.set(42);
        assert_eq!(*state.get(), 42);
        state.modify(|c| *c += 1);
        assert_eq!(*state.get(), 43);
    });
}

#[test]
fn remember_clone_shares_state() {
    // Clone через Arc::clone разделяет один RwLock между всеми клонами.
    // Изменения через любой клон видны во всех остальных.
    with_ui(|ui| {
        let original = remember(ui, "clone_share", || 0i32);
        let clone = original.clone();

        // Изменение через клон видно в оригинале
        clone.set(42);
        assert_eq!(*original.get(), 42, "clone должен разделять состояние");

        // Изменение через оригинал видно в клоне
        original.modify(|c| *c += 1);
        assert_eq!(*clone.get(), 43, "изменения в оригинале видны в клоне");

        // Множественные клоны
        let clone2 = clone.clone();
        clone2.set(100);
        assert_eq!(*original.get(), 100, "все клоны разделяют одно состояние");
        assert_eq!(*clone.get(), 100);
    });
}

#[test]
fn remember_returns_same_arc_between_calls() {
    // Между вызовами remember() возвращается тот же Arc<RwLock<T>>,
    // так как он хранится в IdTypeMap.
    with_ui(|ui| {
        let state1 = remember(ui, "same_arc", || 0i32);
        state1.set(42);

        let state2 = remember(ui, "same_arc", || 0i32);
        assert_eq!(
            *state2.get(),
            42,
            "второй вызов должен вернуть то же состояние"
        );

        state2.modify(|c| *c += 1);
        assert_eq!(*state1.get(), 43, "изменения через state2 видны в state1");
    });
}
