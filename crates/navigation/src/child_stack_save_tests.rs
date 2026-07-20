//! Тесты сохранения/восстановления ChildStack через SavedStack + bincode.

use super::*;
use egui_android_core::PersistentState;
use egui_android_core::{Component, ComponentNode, LifecycleObserver, UiWrapper};
use egui_android_runtime::Dispatcher;
use serde::{Deserialize, Serialize};
use std::any::Any;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct CounterData {
    value: i32,
}

struct CounterComp {
    value: i32,
}

impl CounterComp {
    fn new(v: i32) -> Self {
        Self { value: v }
    }
}

impl LifecycleObserver for CounterComp {}
impl Component for CounterComp {
    type State = ();
    type Message = ();
    fn render(&self, _ui: &mut UiWrapper, _d: &Dispatcher<()>) {}
    fn handle(&mut self, _msg: ()) {}
    fn state(&self) -> &Self::State {
        &()
    }
}

impl PersistentState for CounterComp {
    type State = CounterData;
    fn save(&self) -> Self::State {
        CounterData { value: self.value }
    }
    fn restore(&mut self, s: Self::State) {
        self.value = s.value;
    }
}

// Из-за blanket-impl ComponentNode нельзя переопределить save_state/restore_state.
// Тесты вызывают PersistentState напрямую через хелперы.

fn manual_save(c: &CounterComp) -> Option<Box<dyn Any + Send>> {
    PersistentState::save_to_boxed(c)
}

fn manual_restore(c: &mut CounterComp, state: Box<dyn Any + Send>) {
    PersistentState::restore_from_boxed(c, state);
}

struct CounterFactory;
impl ComponentFactory<String> for CounterFactory {
    fn create(&self, _: String) -> Box<dyn ComponentNode> {
        Box::new(CounterComp::new(0))
    }
}

// ─── Тесты ────────────────────────────────────────────────────────────────

#[test]
fn persistent_state_serialization_roundtrip() {
    let c = CounterComp::new(42);
    let state = manual_save(&c).unwrap();
    let bytes = state.downcast::<Vec<u8>>().unwrap();

    let mut restored = CounterComp::new(0);
    manual_restore(&mut restored, Box::new(*bytes));
    assert_eq!(restored.value, 42);
}

#[test]
fn saved_stack_bincode_roundtrip() {
    let s1 =
        manual_save(&CounterComp::new(7)).and_then(|b| b.downcast::<Vec<u8>>().ok().map(|v| *v));
    let s2 =
        manual_save(&CounterComp::new(777)).and_then(|b| b.downcast::<Vec<u8>>().ok().map(|v| *v));

    let saved = SavedStack {
        items: vec![("home".to_string(), s1), ("details".to_string(), s2)],
    };

    let bytes = bincode::serialize(&saved).unwrap();
    let deser: SavedStack<String> = bincode::deserialize(&bytes).unwrap();

    assert_eq!(deser.items.len(), 2);
    assert_eq!(deser.items[0].0, "home");
    assert_eq!(deser.items[1].0, "details");

    let d0: CounterData = bincode::deserialize(deser.items[0].1.as_ref().unwrap()).unwrap();
    let d1: CounterData = bincode::deserialize(deser.items[1].1.as_ref().unwrap()).unwrap();
    assert_eq!(d0.value, 7);
    assert_eq!(d1.value, 777);
}

#[test]
fn restore_from_saved_creates_correct_structure() {
    let s1 =
        manual_save(&CounterComp::new(55)).and_then(|b| b.downcast::<Vec<u8>>().ok().map(|v| *v));
    let s2 =
        manual_save(&CounterComp::new(99)).and_then(|b| b.downcast::<Vec<u8>>().ok().map(|v| *v));

    let saved = SavedStack {
        items: vec![("first".to_string(), s1), ("second".to_string(), s2)],
    };

    let mut stack: ChildStack<String> = ChildStack::new();
    stack.restore_from_saved(saved, &CounterFactory);

    assert_eq!(stack.len(), 2);
    assert_eq!(stack.active_config(), Some(&"second".to_string()));
    assert!(stack.active().is_some());
}

#[test]
fn restore_empty_saved_clears_stack() {
    let mut stack: ChildStack<String> = ChildStack::new();
    stack.push("old".to_string(), Box::new(CounterComp::new(1)));
    assert!(!stack.is_empty());

    stack.restore_from_saved(SavedStack { items: vec![] }, &CounterFactory);
    assert!(stack.is_empty());
}
