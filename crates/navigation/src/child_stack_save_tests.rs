//! Тесты сохранения/восстановления ChildStack через SavedStack + bincode.

use super::*;
use egui_android_core::{
    Component, ComponentNode, LifecycleObserver, PersistentComponent, PersistentState, UiWrapper,
};
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

// ─── Компонент-аналог StateScreen (через PersistentComponent) ─────────
// Максимально приближен к реальному сценарию showcase:
// StateScreen использует #[derive(Component)] с #[persistent_fields(counter)],
// и в фабрике обёрнут в PersistentComponent::new(StateScreen::new()).
//
// Здесь мы повторяем тот же паттерн вручную, чтобы протестировать
// интеграцию derive-макроса (PersistentState) + PersistentComponent + ChildStack.

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct LikeStateScreenData {
    counter: i32,
    label: String,
}

/// Компонент, идентичный тому, что генерирует #[derive(Component)].
/// Имеет persistent-поля (counter, label) и не-persistent (expanded).
struct LikeStateScreen {
    counter: i32,
    label: String,
    expanded: bool,
}

impl LikeStateScreen {
    fn new() -> Self {
        Self {
            counter: 0,
            label: String::new(),
            expanded: false,
        }
    }
}

impl LifecycleObserver for LikeStateScreen {}
impl Component for LikeStateScreen {
    type State = ();
    type Message = ();
    fn render(&self, _ui: &mut UiWrapper, _d: &Dispatcher<()>) {}
    fn handle(&mut self, _msg: ()) {}
    fn state(&self) -> &Self::State {
        &()
    }
}

impl PersistentState for LikeStateScreen {
    type State = LikeStateScreenData;
    fn save(&self) -> Self::State {
        LikeStateScreenData {
            counter: self.counter,
            label: self.label.clone(),
        }
    }
    fn restore(&mut self, s: Self::State) {
        self.counter = s.counter;
        self.label = s.label;
        // expanded не восстанавливаем — оно не persistent
    }
}

// ─── Вспомогательные типы для тестов ──────────────────────────────────

struct PersistentCounterComp {
    inner: CounterComp,
}

impl PersistentCounterComp {
    fn new(v: i32) -> Self {
        Self {
            inner: CounterComp::new(v),
        }
    }
}

impl LifecycleObserver for PersistentCounterComp {}
impl Component for PersistentCounterComp {
    type State = ();
    type Message = ();
    fn render(&self, _ui: &mut UiWrapper, _d: &Dispatcher<()>) {}
    fn handle(&mut self, _msg: ()) {}
    fn state(&self) -> &Self::State {
        &()
    }
}

impl PersistentState for PersistentCounterComp {
    type State = CounterData;
    fn save(&self) -> Self::State {
        CounterData {
            value: self.inner.value,
        }
    }
    fn restore(&mut self, s: Self::State) {
        self.inner.value = s.value;
    }
}

// Из-за blanket-impl ComponentNode нельзя переопределить save_state/restore_state.
// Тесты вызывают PersistentState напрямую через хелперы,
// а PersistentComponent — для интеграционных тестов.

fn manual_save(c: &CounterComp) -> Option<Box<dyn Any + Send>> {
    PersistentState::save_to_boxed(c)
}

fn manual_restore(c: &mut CounterComp, state: Box<dyn Any + Send>) {
    PersistentState::restore_from_boxed(c, state);
}

/// Фабрика для обычных CounterComp (без сохранения).
struct CounterFactory;
impl ComponentFactory<String> for CounterFactory {
    fn create(&self, _: String) -> Box<dyn ComponentNode> {
        Box::new(CounterComp::new(0))
    }
}

/// Фабрика для PersistentCounterComp (с сохранением через PersistentComponent).
struct PersistentCounterFactory;
impl ComponentFactory<String> for PersistentCounterFactory {
    fn create(&self, _: String) -> Box<dyn ComponentNode> {
        Box::new(PersistentComponent::new(PersistentCounterComp::new(0)))
    }
}

/// Фабрика для LikeStateScreen — аналог ShowcaseFactory::create(Route::State).
struct LikeStateScreenFactory;
impl ComponentFactory<String> for LikeStateScreenFactory {
    fn create(&self, _: String) -> Box<dyn ComponentNode> {
        // Точно как в factory.rs: PersistentComponent::new(StateScreen::new())
        Box::new(PersistentComponent::new(LikeStateScreen::new()))
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

// ─── ИНТЕГРАЦИОННЫЕ ТЕСТЫ: PersistentComponent + ChildStack ──────────────

/// Интеграционный тест: PersistentComponent сохраняет и восстанавливает состояние
/// через ChildStack.save() → ChildStack.restore_from_saved().
///
/// Симулирует полный цикл save/restore как в приложении.
#[test]
fn persistent_component_save_restore_cycle() {
    // Создаём стек с PersistentCounterComp со значением 42
    let mut stack: ChildStack<String> = ChildStack::new();
    let comp = PersistentComponent::new(PersistentCounterComp::new(42));
    stack.push("screen".to_string(), Box::new(comp));

    // Сохраняем стек → сериализуем
    let saved = stack.save();
    assert_eq!(saved.items.len(), 1);
    assert_eq!(saved.items[0].0, "screen");
    // Проверяем что состояние сохранилось (есть bytes)
    assert!(
        saved.items[0].1.is_some(),
        "PersistentComponent должен сохранить состояние"
    );

    // Сериализуем через bincode (как в реальном приложении)
    let bytes = bincode::serialize(&saved).unwrap();

    // Десериализуем и восстанавливаем
    let deserialized: SavedStack<String> = bincode::deserialize(&bytes).unwrap();
    let mut restored_stack: ChildStack<String> = ChildStack::new();
    restored_stack.restore_from_saved(deserialized, &PersistentCounterFactory);

    assert_eq!(restored_stack.len(), 1);
    assert_eq!(restored_stack.active_config(), Some(&"screen".to_string()));

    // Проверяем что состояние восстановилось: достаём компонент и проверяем
    let restored_comp = restored_stack.active().unwrap();
    let persistent: &PersistentComponent<PersistentCounterComp> = restored_comp
        .as_any()
        .downcast_ref::<PersistentComponent<PersistentCounterComp>>()
        .unwrap();
    assert_eq!(
        persistent.inner.inner.value, 42,
        "Состояние должно восстановиться через PersistentComponent"
    );
}

/// Интеграционный тест: смешанный стек (есть и обычные компоненты и с сохранением).
#[test]
fn mixed_stack_save_restore() {
    let mut stack: ChildStack<String> = ChildStack::new();

    // Первый элемент — обычный (без сохранения)
    stack.push("home".to_string(), Box::new(CounterComp::new(10)));

    // Второй элемент — с PersistentComponent (сохраняет значение 99)
    let comp = PersistentComponent::new(PersistentCounterComp::new(99));
    stack.push("details".to_string(), Box::new(comp));

    let saved = stack.save();
    assert_eq!(saved.items.len(), 2);
    // Первый элемент: None (обычный компонент без save_state)
    assert!(
        saved.items[0].1.is_none(),
        "Обычный компонент не сохраняет состояние"
    );
    // Второй элемент: Some(bytes) (PersistentComponent)
    assert!(
        saved.items[1].1.is_some(),
        "PersistentComponent сохраняет состояние"
    );

    // Полный цикл bincode
    let bytes = bincode::serialize(&saved).unwrap();
    let deserialized: SavedStack<String> = bincode::deserialize(&bytes).unwrap();

    // Восстанавливаем — обычный компонент создаётся через CounterFactory (value=0),
    // PersistentCounterComp — через PersistentCounterFactory (value=0 до restore)
    struct MixedFactory;
    impl ComponentFactory<String> for MixedFactory {
        fn create(&self, config: String) -> Box<dyn ComponentNode> {
            if config == "details" {
                Box::new(PersistentComponent::new(PersistentCounterComp::new(0)))
            } else {
                Box::new(CounterComp::new(0))
            }
        }
    }

    let mut restored: ChildStack<String> = ChildStack::new();
    restored.restore_from_saved(deserialized, &MixedFactory);

    assert_eq!(restored.len(), 2);

    // Проверяем восстановленное значение для details
    let active = restored.active().unwrap();
    let persistent: &PersistentComponent<PersistentCounterComp> = active
        .as_any()
        .downcast_ref::<PersistentComponent<PersistentCounterComp>>()
        .unwrap();
    assert_eq!(
        persistent.inner.inner.value, 99,
        "PersistentComponent восстановил значение 99"
    );
}

/// Интеграционный тест: два экрана с PersistentComponent в стеке.
/// Каждый сохраняет своё значение.
#[test]
fn multiple_persistent_components_in_stack() {
    let mut stack: ChildStack<String> = ChildStack::new();

    let comp_a = PersistentComponent::new(PersistentCounterComp::new(10));
    stack.push("a".to_string(), Box::new(comp_a));

    let comp_b = PersistentComponent::new(PersistentCounterComp::new(20));
    stack.push("b".to_string(), Box::new(comp_b));

    let comp_c = PersistentComponent::new(PersistentCounterComp::new(30));
    stack.push("c".to_string(), Box::new(comp_c));

    // Save
    let saved = stack.save();
    assert_eq!(saved.items.len(), 3);
    // Все три сохранили состояние
    for (i, (config, _)) in saved.items.iter().enumerate() {
        assert!(
            saved.items[i].1.is_some(),
            "{} должен сохранить состояние",
            config
        );
    }

    // Полный цикл
    let bytes = bincode::serialize(&saved).unwrap();
    let deserialized: SavedStack<String> = bincode::deserialize(&bytes).unwrap();

    let mut restored: ChildStack<String> = ChildStack::new();
    restored.restore_from_saved(deserialized, &PersistentCounterFactory);

    assert_eq!(restored.len(), 3);

    // Проверяем каждый элемент
    // К сожалению, доступ есть только к активному (верхнему)
    // Но можем проверить, что структура стека корректна
    assert_eq!(restored.active_config(), Some(&"c".to_string()));

    // Pop и проверка
    let (config, comp) = restored.pop().unwrap();
    assert_eq!(config, "c");
    let persistent: &PersistentComponent<PersistentCounterComp> = comp
        .as_any()
        .downcast_ref::<PersistentComponent<PersistentCounterComp>>()
        .unwrap();
    assert_eq!(persistent.inner.inner.value, 30);

    let (config, comp) = restored.pop().unwrap();
    assert_eq!(config, "b");
    let persistent: &PersistentComponent<PersistentCounterComp> = comp
        .as_any()
        .downcast_ref::<PersistentComponent<PersistentCounterComp>>()
        .unwrap();
    assert_eq!(persistent.inner.inner.value, 20);

    let (config, comp) = restored.pop().unwrap();
    assert_eq!(config, "a");
    let persistent: &PersistentComponent<PersistentCounterComp> = comp
        .as_any()
        .downcast_ref::<PersistentComponent<PersistentCounterComp>>()
        .unwrap();
    assert_eq!(persistent.inner.inner.value, 10);
}

/// Юнит-тест: PersistentComponent.save_state() возвращает сериализованные данные.
#[test]
fn persistent_component_save_state_returns_bytes() {
    let comp = PersistentComponent::new(PersistentCounterComp::new(77));

    let saved = comp.save_state();
    assert!(saved.is_some());

    let boxed = saved.unwrap();
    let bytes = boxed.downcast::<Vec<u8>>().unwrap();

    // Десериализуем и проверяем значение
    let data: CounterData = bincode::deserialize(&bytes).unwrap();
    assert_eq!(data.value, 77);
}

/// Юнит-тест: PersistentComponent.restore_state() восстанавливает данные.
#[test]
fn persistent_component_restore_state_restores_value() {
    let mut comp = PersistentComponent::new(PersistentCounterComp::new(0));

    // Сохраняем состояние с value=55
    let saved = PersistentState::save_to_boxed(&PersistentCounterComp::new(55)).unwrap();

    // Восстанавливаем
    comp.restore_state(saved);

    assert_eq!(comp.inner.inner.value, 55);
}

// ─── ИНТЕГРАЦИОННЫЙ ТЕСТ: Сценарий StateScreen из showcase ─────────────

/// Интеграционный тест: полный цикл save/restore для компонента,
/// идентичного StateScreen (через #[derive(Component)] + PersistentComponent).
///
/// Это точная копия сценария из ShowcaseApplication:
/// 1. Route::State → фабрика создаёт PersistentComponent::new(StateScreen::new())
/// 2. Пользователь меняет counter → 42, label → "test"
/// 3. on_save_state() → ChildStack::save() → сериализация
/// 4. on_restore_state() → ChildStack::restore_from_saved() → состояние восстановлено
#[test]
fn state_screen_like_save_restore_via_persistent_component() {
    // Шаг 1: Фабрика создаёт компонент — как в ShowcaseFactory
    let factory = LikeStateScreenFactory;
    let mut screen: Box<dyn ComponentNode> = factory.create("state".to_string());

    // Шаг 2: Пользователь меняет состояние (как в StateScreen::handle)
    // Достаём внутренний тип через as_any_mut и меняем поля
    // as_any_mut() на dyn ComponentNode (PersistentComponent) возвращает &mut self
    let persistent: &mut PersistentComponent<LikeStateScreen> = screen
        .as_any_mut()
        .downcast_mut::<PersistentComponent<LikeStateScreen>>()
        .expect("PersistentComponent<LikeStateScreen>");
    let inner = &mut persistent.inner;
    inner.counter = 42;
    inner.label = "test_label".to_string();
    inner.expanded = true; // не-persistent поле, не должно восстановиться

    // Шаг 3: on_save_state — сохраняем через ChildStack (как в приложении)
    let mut stack: ChildStack<String> = ChildStack::new();
    stack.push("state".to_string(), screen);

    let saved = stack.save();
    // Проверяем что save вернул bytes
    assert!(
        saved.items[0].1.is_some(),
        "PersistentComponent должен сохранить bytes"
    );

    let bytes = bincode::serialize(&saved).unwrap();

    // Шаг 4: on_restore_state — восстанавливаем
    let deserialized: SavedStack<String> = bincode::deserialize(&bytes).unwrap();
    let mut restored_stack: ChildStack<String> = ChildStack::new();
    restored_stack.restore_from_saved(deserialized, &LikeStateScreenFactory);

    assert_eq!(restored_stack.len(), 1);

    // Проверяем что persistent-поля восстановились
    let restored = restored_stack.active().unwrap();
    let persistent: &PersistentComponent<LikeStateScreen> = restored
        .as_any()
        .downcast_ref::<PersistentComponent<LikeStateScreen>>()
        .expect("PersistentComponent<LikeStateScreen>");

    assert_eq!(
        persistent.inner.counter, 42,
        "counter должен восстановиться (persistent поле)"
    );
    assert_eq!(
        persistent.inner.label, "test_label",
        "label должен восстановиться (persistent поле)"
    );
    // expanded — не-persistent поле, должно быть значением по умолчанию (false),
    // потому что компонент создаётся заново через factory.create()
    assert!(
        !persistent.inner.expanded,
        "expanded не должен восстановиться (не persistent поле)"
    );
}

/// Интеграционный тест: save/restore через Application-подобный цикл.
///
/// Максимально приближен к реальному ShowcaseApplication:
/// - on_save_state: root.save() → bincode → сохраняем
/// - on_restore_state: bincode → root.restore()
/// - После restore проверяем, что persistent-поля корректны
#[test]
fn application_like_save_restore_cycle() {
    // Создаём стек с LikeStateScreen (аналог Route::State)
    let mut stack: ChildStack<String> = ChildStack::new();

    let screen = PersistentComponent::new(LikeStateScreen::new());
    stack.push("state".to_string(), Box::new(screen));

    // Меняем состояние активного компонента (как пользователь через handle)
    {
        let active = stack.active_mut().unwrap();
        let persistent: &mut PersistentComponent<LikeStateScreen> = active
            .as_any_mut()
            .downcast_mut::<PersistentComponent<LikeStateScreen>>()
            .unwrap();
        let inner = &mut persistent.inner;
        inner.counter = 100;
        inner.label = "app_cycle".to_string();
    }

    // on_save_state: как в ShowcaseApplication
    let saved = stack.save();
    let bytes = bincode::serialize(&saved).expect("bincode serialize");

    // on_restore_state: как в ShowcaseApplication
    let deserialized: SavedStack<String> =
        bincode::deserialize(&bytes).expect("bincode deserialize");
    let mut restored_stack: ChildStack<String> = ChildStack::new();
    restored_stack.restore_from_saved(deserialized, &LikeStateScreenFactory);

    // Проверяем восстановление
    let restored = restored_stack.active().unwrap();
    let persistent: &PersistentComponent<LikeStateScreen> = restored
        .as_any()
        .downcast_ref::<PersistentComponent<LikeStateScreen>>()
        .unwrap();

    assert_eq!(persistent.inner.counter, 100, "counter=100 после restore");
    assert_eq!(
        persistent.inner.label, "app_cycle",
        "label='app_cycle' после restore"
    );
    assert!(
        !persistent.inner.expanded,
        "expanded=false (по умолчанию, не persistent)"
    );
}
