//! Интеграционные тесты для derive-макроса Component.
//!
//! Проверяют генерацию PersistentState через #[derive(Component)] + #[persistent_fields(...)].

use egui_android_framework::core::{
    Component as EguiComponent, ComponentNode, LifecycleObserver, PersistentState, UiWrapper,
};
use egui_android_framework::runtime::Dispatcher;
use egui_android_macros::Component;

// ─── Компонент с persistent полями ─────────────────────────────────

#[derive(Clone, Debug)]
enum TestMsg {
    Increment,
}

#[derive(Component)]
#[persistent_fields(counter, label)]
struct TestScreen {
    counter: i32,
    label: String,
    expanded: bool,
}

impl TestScreen {
    fn new() -> Self {
        Self {
            counter: 0,
            label: String::from("default"),
            expanded: false,
        }
    }
}

impl LifecycleObserver for TestScreen {}

impl EguiComponent for TestScreen {
    type State = ();
    type Message = TestMsg;

    fn render(&self, _ui: &mut UiWrapper, _d: &Dispatcher<TestMsg>) {}
    fn handle(&mut self, _msg: TestMsg) {}
    fn state(&self) -> &Self::State {
        &()
    }
}

// ─── Тесты ─────────────────────────────────────────────────────────

#[test]
fn save_returns_persistent_fields() {
    let mut screen = TestScreen::new();
    screen.counter = 42;
    screen.label = String::from("hello");
    screen.expanded = true;

    let saved = PersistentState::save(&screen);
    assert_eq!(saved.counter, 42);
    assert_eq!(saved.label, "hello");
}

#[test]
fn restore_sets_persistent_fields() {
    let mut screen = TestScreen::new();
    screen.expanded = true;

    let saved = __TestScreenPersistentState {
        counter: 99,
        label: String::from("restored"),
    };
    PersistentState::restore(&mut screen, saved);

    assert_eq!(screen.counter, 99);
    assert_eq!(screen.label, "restored");
    assert_eq!(screen.expanded, true); // не перезаписалось
}

#[test]
fn save_restore_roundtrip_via_helpers() {
    let mut screen = TestScreen::new();
    screen.counter = 77;
    screen.label = String::from("roundtrip");

    let saved = PersistentState::save_to_boxed(&screen);
    assert!(saved.is_some(), "save_to_boxed должен вернуть Some");

    let mut restored = TestScreen::new();
    PersistentState::restore_from_boxed(&mut restored, saved.unwrap());

    assert_eq!(restored.counter, 77);
    assert_eq!(restored.label, "roundtrip");
}

#[test]
fn generate_serialize_deserialize_roundtrip() {
    let mut screen = TestScreen::new();
    screen.counter = 55;
    screen.label = String::from("bincode");

    let saved = PersistentState::save(&screen);
    let bytes = bincode::serialize(&saved).expect("serialize");
    let deser: __TestScreenPersistentState = bincode::deserialize(&bytes).expect("deserialize");

    assert_eq!(deser.counter, 55);
    assert_eq!(deser.label, "bincode");
}
