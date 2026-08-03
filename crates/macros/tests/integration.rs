//! Интеграционные тесты для derive-макросов.
//!
//! Проверяют генерацию PersistentState через #[derive(PersistentState)]
//! и ComponentNode через #[derive(ComponentNode)] (включая back_message/back_handler).

use egui_android_core::{
    BackAction, Component as EguiComponent, ComponentContext, LifecycleObserver, PersistentState,
    UiWrapper,
};
use egui_android_macros::{ComponentNode, PersistentState};
use egui_android_runtime::Dispatcher;

// ─── Компонент с persistent полями ─────────────────────────────────

#[derive(Clone, Debug)]
enum TestMsg {
    Increment,
}

#[derive(PersistentState)]
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

    fn render(&self, _ui: &mut UiWrapper, _d: &Dispatcher<TestMsg>, _ctx: &ComponentContext) {}
    fn handle(&mut self, _msg: TestMsg, _ctx: &mut ComponentContext) {}
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

// ─── Компонент с кастомным Back (#[back_message] + #[back_handler]) ──

/// Сообщения экрана с кастомным Back.
#[derive(Clone, Debug, PartialEq)]
enum CounterMsg {
    Increment,
    Reset,
    Back,
}

/// Компонент, где handle_back генерируется макросом и делегирует в метод on_back.
#[derive(ComponentNode)]
#[component_message(CounterMsg)]
#[back_message(CounterMsg::Back)]
#[back_handler(on_back)]
struct BackComponent {
    counter: i32,
}

impl BackComponent {
    fn new() -> Self {
        Self { counter: 0 }
    }

    /// Логика Back: сбрасывает счётчик и просит pop.
    fn on_back(&mut self, _ctx: &mut ComponentContext) -> BackAction {
        self.counter = 0;
        BackAction::Pop
    }
}

impl LifecycleObserver for BackComponent {}

impl EguiComponent for BackComponent {
    type State = ();
    type Message = CounterMsg;

    fn render(&self, _ui: &mut UiWrapper, _d: &Dispatcher<CounterMsg>, _ctx: &ComponentContext) {}
    fn handle(&mut self, msg: CounterMsg, _ctx: &mut ComponentContext) {
        match msg {
            CounterMsg::Increment => self.counter += 1,
            CounterMsg::Reset => self.counter = 0,
            // Back обрабатывается через сгенерированный handle_back (не здесь).
            CounterMsg::Back => {}
        }
    }
    fn state(&self) -> &Self::State {
        &()
    }
}

/// handle_back, сгенерированный через #[back_handler], должен делегировать в on_back.
#[test]
fn back_handler_delegates_to_method() {
    let mut comp = BackComponent::new();
    comp.counter = 42;
    let mut ctx = ComponentContext::new();

    // Вызываем handle_back от арены (как делает ChildStack::on_back).
    let action = egui_android_core::ComponentNode::handle_back(&mut comp, &mut ctx);

    assert_eq!(
        action,
        BackAction::Pop,
        "handle_back должен вернуть Pop из on_back"
    );
    assert_eq!(comp.counter, 0, "on_back должен сбросить счётчик");
}

/// handle_dyn для Back-варианта должен вернуть Some(Propagate), а не None.
#[test]
fn back_message_makes_handle_dyn_propagate() {
    let mut comp = BackComponent::new();
    comp.counter = 5;
    let mut ctx = ComponentContext::new();

    let action = egui_android_core::ComponentNode::handle_dyn(
        &mut comp,
        Box::new(CounterMsg::Back),
        &mut ctx,
    );

    assert_eq!(
        action,
        Some(BackAction::Propagate),
        "handle_dyn для Back должен вернуть Some(Propagate)"
    );
    assert_eq!(
        comp.counter, 5,
        "handle_dyn НЕ должен вызывать on_back (навигация передаётся в ChildStack)"
    );
}

/// handle_dyn для обычного сообщения должен вернуть None.
#[test]
fn back_message_keeps_ordinary_messages_none() {
    let mut comp = BackComponent::new();
    comp.counter = 1;
    let mut ctx = ComponentContext::new();

    let action = egui_android_core::ComponentNode::handle_dyn(
        &mut comp,
        Box::new(CounterMsg::Increment),
        &mut ctx,
    );

    assert_eq!(action, None, "обычное сообщение должно вернуть None");
    assert_eq!(comp.counter, 2, "Increment должен применить handle()");
}
