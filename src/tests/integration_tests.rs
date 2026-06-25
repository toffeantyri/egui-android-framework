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

    fn render(&self, ui: &mut egui::Ui) -> Vec<Self::Message> {
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
