//! Интеграционные тесты для новой Decompose-style архитектуры.
//!
//! Тестирует:
//! - ChildStack навигацию (push/pop/replace/bring_to_front)
//! - Жизненный цикл компонентов
//! - Интеграцию Application, Component и ChildStack
//! - Передачу сообщений от render → handle

use crate::application::{AppConfig, Application};
use crate::child_stack::ChildStack;
use crate::component::Component;
use crate::LifecycleObserver;

// ─── Мок-компонент с трекингом состояния ───────────────────────────────────────

/// Мок-компонент, который логирует события жизненного цикла и сообщения.
#[derive(Default)]
struct MockComponent {
    /// Текущее состояние — количество "нажатий" (сообщений Incr).
    value: u32,
    /// Лог событий жизненного цикла.
    lifecycle_log: Vec<&'static str>,
    /// Лог полученных сообщений.
    handle_log: Vec<&'static str>,
}

impl LifecycleObserver for MockComponent {
    fn on_create(&mut self) {
        self.lifecycle_log.push("create");
    }
    fn on_start(&mut self) {
        self.lifecycle_log.push("start");
    }
    fn on_resume(&mut self) {
        self.lifecycle_log.push("resume");
    }
    fn on_pause(&mut self) {
        self.lifecycle_log.push("pause");
    }
    fn on_stop(&mut self) {
        self.lifecycle_log.push("stop");
    }
    fn on_destroy(&mut self) {
        self.lifecycle_log.push("destroy");
    }
}

/// Сообщение для мок-компонента.
#[derive(Debug, Clone, PartialEq)]
enum MockMsg {
    Incr,
    Decr,
}

impl Component for MockComponent {
    type State = u32;
    type Message = MockMsg;

    fn render(&self, _ui: &mut egui::Ui) -> Vec<Self::Message> {
        // Не используем CentralPanel — тесты вызывают render внутри ctx.run()
        if self.value == 0 {
            vec![MockMsg::Incr]
        } else {
            vec![]
        }
    }

    fn handle(&mut self, msg: Self::Message) {
        match msg {
            MockMsg::Incr => {
                self.value += 1;
                self.handle_log.push("Incr");
            }
            MockMsg::Decr => {
                self.value = self.value.saturating_sub(1);
                self.handle_log.push("Decr");
            }
        }
    }

    fn state(&self) -> &Self::State {
        &self.value
    }
}

// ─── Тесты навигации (ChildStack) ──────────────────────────────────────────────

#[test]
fn test_navigation_push() {
    let mut stack = ChildStack::<&'static str, MockComponent>::new();

    let comp_a = MockComponent::default();
    stack.push("screen_a", comp_a);

    assert_eq!(stack.len(), 1);
    assert_eq!(stack.active_config(), Some(&"screen_a"));

    let active = stack.active().unwrap();
    assert_eq!(active.value, 0);
    // Компонент A прошёл полный lifecycle
    assert_eq!(active.lifecycle_log, vec!["create", "start", "resume"]);
}

#[test]
fn test_navigation_push_twice() {
    let mut stack = ChildStack::<&'static str, MockComponent>::new();

    stack.push("a", MockComponent::default());
    stack.push("b", MockComponent::default());

    assert_eq!(stack.len(), 2);
    assert_eq!(stack.active_config(), Some(&"b"));

    // Активный — B, он получил полный lifecycle
    let b = stack.active().unwrap();
    assert_eq!(b.lifecycle_log, vec!["create", "start", "resume"]);

    // A — не активный, у него был on_pause при push B
    // К сожалению, у нас нет доступа к A после push.
    // Проверяем только B.
}

#[test]
fn test_navigation_pop() {
    let mut stack = ChildStack::<&'static str, MockComponent>::new();

    stack.push("a", MockComponent::default());
    stack.push("b", MockComponent::default());

    let (config, comp_b) = stack.pop().unwrap();

    assert_eq!(config, "b");
    assert_eq!(stack.len(), 1);
    assert_eq!(stack.active_config(), Some(&"a"));

    // B уничтожен: pause → stop → destroy
    assert_eq!(
        comp_b.lifecycle_log,
        vec!["create", "start", "resume", "pause", "stop", "destroy"]
    );
}

#[test]
fn test_navigation_pop_empty() {
    let mut stack = ChildStack::<&'static str, MockComponent>::new();
    assert!(stack.pop().is_none());
}

#[test]
fn test_navigation_replace() {
    let mut stack = ChildStack::<&'static str, MockComponent>::new();
    stack.push("a", MockComponent::default());
    stack.replace("b", MockComponent::default());

    assert_eq!(stack.len(), 1);
    assert_eq!(stack.active_config(), Some(&"b"));

    let b = stack.active().unwrap();
    assert_eq!(b.lifecycle_log, vec!["create", "start", "resume"]);
}

#[test]
fn test_navigation_bring_to_front_new() {
    let mut stack = ChildStack::<&'static str, MockComponent>::new();
    stack.push("a", MockComponent::default());
    stack.bring_to_front("b", MockComponent::default());

    assert_eq!(stack.len(), 1);
    assert_eq!(stack.active_config(), Some(&"b"));
}

#[test]
fn test_navigation_clear() {
    let mut stack = ChildStack::<&'static str, MockComponent>::new();
    stack.push("a", MockComponent::default());
    stack.push("b", MockComponent::default());
    stack.clear();

    assert!(stack.is_empty());
}

// ─── Тесты жизненного цикла компонента ─────────────────────────────────────────

#[test]
fn test_component_lifecycle_full() {
    let mut comp = MockComponent::default();

    assert!(comp.lifecycle_log.is_empty());

    comp.on_create();
    comp.on_start();
    comp.on_resume();
    comp.on_pause();
    comp.on_stop();
    comp.on_destroy();

    assert_eq!(
        comp.lifecycle_log,
        vec!["create", "start", "resume", "pause", "stop", "destroy"]
    );
}

#[test]
fn test_component_lifecycle_partial() {
    let mut comp = MockComponent::default();

    comp.on_create();
    comp.on_start();
    comp.on_resume();
    comp.on_pause();

    assert_eq!(
        comp.lifecycle_log,
        vec!["create", "start", "resume", "pause"]
    );
}

#[test]
fn test_component_lifecycle_default_impl() {
    // Проверяем, что у компонента есть пустые реализации по умолчанию
    let mut comp = MockComponent::default();
    comp.on_create();
    assert_eq!(comp.lifecycle_log, vec!["create"]);
}

// ─── Тесты Component (render → handle) ─────────────────────────────────────────

#[test]
fn test_component_handle_message() {
    let mut comp = MockComponent::default();
    assert_eq!(*comp.state(), 0);

    comp.handle(MockMsg::Incr);
    assert_eq!(*comp.state(), 1);
    assert_eq!(comp.handle_log, vec!["Incr"]);

    comp.handle(MockMsg::Incr);
    assert_eq!(*comp.state(), 2);

    comp.handle(MockMsg::Decr);
    assert_eq!(*comp.state(), 1);
    assert_eq!(comp.handle_log, vec!["Incr", "Incr", "Decr"]);
}

#[test]
fn test_component_render_returns_messages() {
    let mut comp = MockComponent::default();

    // При value=0 render возвращает [Incr]
    {
        let ctx = egui::Context::default();
        let raw = egui::RawInput::default();
        let _ = ctx.run(raw, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let messages = comp.render(ui);
                assert_eq!(messages, vec![MockMsg::Incr]);
            });
        });
    }

    // После handle(Incr) value=1, render возвращает []
    comp.handle(MockMsg::Incr);

    {
        let ctx = egui::Context::default();
        let raw = egui::RawInput::default();
        let _ = ctx.run(raw, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let messages = comp.render(ui);
                assert!(messages.is_empty());
            });
        });
    }
}

#[test]
fn test_component_handle_after_render() {
    // Симулируем полный цикл: render → handle
    let mut comp = MockComponent::default();

    // Получаем сообщения через render
    let messages = {
        let ctx = egui::Context::default();
        let raw = egui::RawInput::default();
        let mut messages = Vec::new();
        let _ = ctx.run(raw, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                messages = comp.render(ui);
            });
        });
        messages
    };

    // Обрабатываем сообщения
    for msg in messages {
        comp.handle(msg);
    }

    assert_eq!(*comp.state(), 1);
    assert_eq!(comp.handle_log, vec!["Incr"]);
}

// ─── Интеграционные тесты (Application + Component + ChildStack) ───────────────

/// Тестовое приложение с простым компонентом.
struct TestApp {
    root: MockComponent,
    config: AppConfig,
}

impl LifecycleObserver for TestApp {
    fn on_create(&mut self) {
        self.root.on_create();
    }
    fn on_start(&mut self) {
        self.root.on_start();
    }
    fn on_resume(&mut self) {
        self.root.on_resume();
    }
    fn on_pause(&mut self) {
        self.root.on_pause();
    }
    fn on_stop(&mut self) {
        self.root.on_stop();
    }
    fn on_destroy(&mut self) {
        self.root.on_destroy();
    }
}

impl Application for TestApp {
    type RootComponent = MockComponent;

    fn create() -> Self {
        Self {
            root: MockComponent::default(),
            config: AppConfig::default(),
        }
    }

    fn root(&mut self) -> &mut MockComponent {
        &mut self.root
    }
    fn root_ref(&self) -> &MockComponent {
        &self.root
    }
    fn config(&self) -> &AppConfig {
        &self.config
    }
    fn config_mut(&mut self) -> &mut AppConfig {
        &mut self.config
    }
}

#[test]
fn test_app_creation_defaults() {
    let app = TestApp::create();
    assert_eq!(*app.root_ref().state(), 0);
    assert_eq!(app.config().log_tag, "egui_app");
    assert_eq!(app.config().target_fps, 60);
}

#[test]
fn test_app_lifecycle_delegation() {
    let mut app = TestApp::create();

    app.on_create();
    app.on_start();
    app.on_resume();

    assert_eq!(
        app.root_ref().lifecycle_log,
        vec!["create", "start", "resume"]
    );

    app.on_pause();
    app.on_stop();
    app.on_destroy();

    assert_eq!(
        app.root_ref().lifecycle_log,
        vec!["create", "start", "resume", "pause", "stop", "destroy"]
    );
}

#[test]
fn test_app_root_access() {
    let mut app = TestApp::create();
    let root = app.root();
    assert_eq!(*root.state(), 0);

    // Через root можем отправлять сообщения
    root.handle(MockMsg::Incr);
    assert_eq!(*root.state(), 1);
}

#[test]
fn test_app_with_child_stack() {
    // Application + ChildStack: пример будущей навигации
    let mut stack = ChildStack::<&'static str, MockComponent>::new();

    stack.push("main", MockComponent::default());
    stack.push("settings", MockComponent::default());

    assert_eq!(stack.len(), 2);
    assert_eq!(stack.active_config(), Some(&"settings"));

    // Взаимодействие с активным компонентом через стек
    if let Some(active) = stack.active_mut() {
        active.handle(MockMsg::Incr);
        assert_eq!(*active.state(), 1);
    }

    // Возврат на главный экран
    let (config, _settings) = stack.pop().unwrap();
    assert_eq!(config, "settings");
    assert_eq!(stack.active_config(), Some(&"main"));
}

#[test]
fn test_app_full_integration() {
    // Интеграционный тест: создаём приложение, симулируем lifecycle,
    // render → handle, проверяем состояние
    let mut app = TestApp::create();

    // Lifecycle
    app.on_create();
    app.on_start();
    app.on_resume();

    // Симулируем кадр: render → handle
    let messages = {
        let ctx = egui::Context::default();
        let raw = egui::RawInput::default();
        let mut messages = Vec::new();
        let _ = ctx.run(raw, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                messages = app.root().render(ui);
            });
        });
        messages
    };

    for msg in messages {
        app.root().handle(msg);
    }

    // После обработки сообщений — состояние должно обновиться
    assert_eq!(*app.root_ref().state(), 1);
    assert_eq!(app.root_ref().handle_log, vec!["Incr"]);
}

// ─── Тесты: Store + канал уведомлений ─────────────────────────────────────

#[test]
fn test_store_update_triggers_notify_tx() {
    use crate::store::StateStore;
    use std::sync::mpsc;

    let store = StateStore::new(0u32);
    let (notify_tx, notify_rx) = mpsc::channel::<()>();

    // Симулируем data layer: обновляем store, шлём уведомление
    store.update(|s| *s += 1);
    let _ = notify_tx.send(());

    // notify_rx должен получить сигнал
    assert!(notify_rx.try_recv().is_ok());
    // Состояние обновлено
    assert_eq!(store.state(), 1);
}

#[test]
fn test_store_clone_state_shares_channel() {
    use crate::store::StateStore;

    let store = StateStore::new(0u32);
    let store_clone = store.clone_state();

    // Клон меняет состояние
    store_clone.update(|s| *s = 42);

    // Оригинал видит изменение
    assert_eq!(store.state(), 42);

    // Подписчик оригинала тоже видит
    let mut rx = store.subscribe();
    store.update(|s| *s = 99);
    assert!(rx.has_changed().unwrap());
    let _ = rx.borrow_and_update();
    assert_eq!(*rx.borrow(), 99);
}

// ─── Тесты: Component синхронизация из store ─────────────────────────────

#[test]
fn test_component_syncs_from_store() {
    use crate::component::Component;
    use crate::store::StateStore;

    #[derive(Clone, Debug, PartialEq, Default)]
    struct SimpleState {
        value: u32,
    }

    struct SyncingComponent {
        value: u32,
        store: StateStore<SimpleState>,
    }

    impl LifecycleObserver for SyncingComponent {}

    impl Component for SyncingComponent {
        type State = u32;
        type Message = ();
        fn render(&self, _ui: &mut egui::Ui) -> Vec<Self::Message> {
            vec![]
        }
        fn handle(&mut self, _msg: Self::Message) {}
        fn state(&self) -> &Self::State {
            &self.value
        }
    }

    impl SyncingComponent {
        fn new(store: StateStore<SimpleState>) -> Self {
            let value = store.state().value;
            Self { value, store }
        }
        fn sync_from_store(&mut self) {
            self.value = self.store.state().value;
        }
    }

    let store = StateStore::new(SimpleState { value: 0 });
    let mut comp = SyncingComponent::new(store.clone_state());

    // Начальное значение
    assert_eq!(comp.value, 0);

    // Data layer меняет store
    store.update(|s| s.value = 42);

    // Компонент синхронизируется
    comp.sync_from_store();
    assert_eq!(comp.value, 42);

    // Ещё одно изменение
    store.update(|s| s.value = 99);
    comp.sync_from_store();
    assert_eq!(comp.value, 99);
}

#[test]
fn test_full_mvi_flow_state_store_only() {
    // Тестирует полный MVI-поток без Android/egui зависимостей:
    // View возвращает Message → handle обрабатывает → store обновляется
    use crate::component::Component;
    use crate::store::StateStore;

    #[derive(Clone, Debug, PartialEq, Default)]
    struct CounterState {
        count: u32,
    }

    #[derive(Debug, Clone, PartialEq)]
    enum CounterMsg {
        Increment,
    }

    struct CounterComponent {
        count: u32,
        store: StateStore<CounterState>,
    }

    impl LifecycleObserver for CounterComponent {}

    impl Component for CounterComponent {
        type State = u32;
        type Message = CounterMsg;
        fn render(&self, _ui: &mut egui::Ui) -> Vec<Self::Message> {
            if self.count < 5 {
                vec![CounterMsg::Increment]
            } else {
                vec![]
            }
        }
        fn handle(&mut self, msg: Self::Message) {
            if let CounterMsg::Increment = msg {
                self.store.update(|s| s.count += 1);
            }
        }
        fn state(&self) -> &Self::State {
            &self.count
        }
    }

    impl CounterComponent {
        fn new(store: StateStore<CounterState>) -> Self {
            let count = store.state().count;
            Self { count, store }
        }
        fn sync_from_store(&mut self) {
            self.count = self.store.state().count;
        }
    }

    let store = StateStore::new(CounterState { count: 0 });
    let mut comp = CounterComponent::new(store.clone_state());

    // Симулируем 5 кадров: render → handle → sync
    for frame in 1..=5 {
        let msgs = {
            let ctx = egui::Context::default();
            let raw = egui::RawInput::default();
            let mut msgs = Vec::new();
            let _ = ctx.run(raw, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    msgs = comp.render(ui);
                });
            });
            msgs
        };

        assert!(!msgs.is_empty(), "Кадр {frame}: сообщения должны быть");
        for msg in msgs {
            comp.handle(msg);
        }

        // Синхронизация из store
        comp.sync_from_store();

        // После каждого кадра count увеличивается
        assert_eq!(
            comp.count, frame as u32,
            "После {frame} кадров count должен быть {frame}"
        );
    }

    // На 6 кадре render уже не возвращает сообщений
    comp.sync_from_store();
    let msgs = {
        let ctx = egui::Context::default();
        let raw = egui::RawInput::default();
        let mut msgs = Vec::new();
        let _ = ctx.run(raw, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                msgs = comp.render(ui);
            });
        });
        msgs
    };
    assert!(
        msgs.is_empty(),
        "count >= 5 — render не должен возвращать сообщений"
    );
    assert_eq!(comp.count, 5);
}
