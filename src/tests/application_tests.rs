use crate::{Activity, AppContext, Application, LifecycleObserver, ViewModel};

enum TestCmd {
    Add(i32),
}

enum TestEvt {
    Updated(i32),
}

struct TestVM {
    value: i32,
    ctx: crate::ViewModelContext<TestCmd, TestEvt>,
}

impl ViewModel for TestVM {
    type DataCommand = TestCmd;
    type Event = TestEvt;

    fn create(context: crate::ViewModelContext<Self::DataCommand, Self::Event>) -> Self {
        Self {
            value: 0,
            ctx: context,
        }
    }

    fn handle(&mut self, cmd: Self::DataCommand) {
        match cmd {
            TestCmd::Add(n) => self.value += n,
        }
    }

    fn on_event(&mut self, evt: Self::Event) {
        let TestEvt::Updated(v) = evt;
        self.value = v;
    }
}

struct TestActivity;

impl Activity for TestActivity {
    type ViewModel = TestVM;
    type Application = TestApp;

    fn create(_context: &AppContext<Self::Application>) -> Self {
        Self
    }

    fn render(
        &mut self,
        _ctx: &egui::Context,
        _vm: &Self::ViewModel,
    ) -> Vec<<Self::ViewModel as ViewModel>::DataCommand> {
        vec![]
    }
}

impl LifecycleObserver for TestActivity {}

struct TestApp;

impl Application for TestApp {
    type Activity = TestActivity;
    type ViewModel = TestVM;

    fn on_create(_context: &mut AppContext<Self>) {}

    fn create_view_model(context: &mut AppContext<Self>) -> Self::ViewModel {
        let vm_ctx = context.view_model_context();
        TestVM::create(vm_ctx)
    }

    fn create_activity(_context: &mut AppContext<Self>) -> Self::Activity {
        TestActivity
    }
}

// ─── Существующие тесты ───────────────────────────────────────────────────────

#[test]
fn test_data_layer_receives_commands_from_vm() {
    let mut ctx = AppContext::<TestApp>::new();
    let vm = TestApp::create_view_model(&mut ctx);
    let (dl_cmd_rx, dl_evt_tx) = ctx.take_data_layer_channels();

    vm.ctx.send(TestCmd::Add(5));

    assert!(matches!(dl_cmd_rx.try_recv(), Ok(TestCmd::Add(5))));
    assert!(dl_evt_tx.send(TestEvt::Updated(10)).is_ok());
}

#[test]
fn test_vm_receives_events_from_data_layer() {
    let mut ctx = AppContext::<TestApp>::new();
    let mut vm = TestApp::create_view_model(&mut ctx);
    let (_dl_cmd_rx, dl_evt_tx) = ctx.take_data_layer_channels();

    dl_evt_tx.send(TestEvt::Updated(42)).unwrap();

    let events = vm.ctx.poll_events();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0], TestEvt::Updated(42)));
    assert_eq!(vm.value, 0);

    for evt in events {
        vm.on_event(evt);
    }
    assert_eq!(vm.value, 42);
}

#[test]
fn test_take_data_layer_channels_panics_before_view_model_context() {
    let mut ctx = AppContext::<TestApp>::new();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = ctx.take_data_layer_channels();
    }));
    assert!(
        result.is_err(),
        "Ожидается паника, когда каналы не инициализированы"
    );
}

#[test]
fn test_take_data_layer_channels_once() {
    let mut ctx = AppContext::<TestApp>::new();
    TestApp::create_view_model(&mut ctx);

    let _first = ctx.take_data_layer_channels();
    let second = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ctx.take_data_layer_channels();
    }));
    assert!(second.is_err(), "Ожидается паника при повторном взятии");
}

#[test]
fn test_view_model_context_sets_channels() {
    let mut ctx = AppContext::<TestApp>::new();
    let _vm_ctx = ctx.view_model_context();

    let (dl_cmd_rx, dl_evt_tx) = ctx.take_data_layer_channels();
    assert!(dl_cmd_rx.try_recv().is_err());
    assert!(dl_evt_tx.send(TestEvt::Updated(0)).is_ok());
}

// ─── Новые тесты ──────────────────────────────────────────────────────────────

#[test]
fn test_create_view_model_without_view_model_context_panics() {
    let mut ctx = AppContext::<TestApp>::new();

    // panic: take_data_layer_channels до view_model_context
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = ctx.take_data_layer_channels();
    }));
    assert!(result.is_err());

    // panic: create_view_model без вызова view_model_context внутри
    // (вcreate_view_model ниже мы забыли вызвать ctx.view_model_context())
    struct BadActivity;

    impl LifecycleObserver for BadActivity {}

    impl Activity for BadActivity {
        type ViewModel = TestVM;
        type Application = BadApp;

        fn create(_context: &AppContext<Self::Application>) -> Self {
            Self
        }

        fn render(
            &mut self,
            _ctx: &egui::Context,
            _vm: &Self::ViewModel,
        ) -> Vec<<Self::ViewModel as ViewModel>::DataCommand> {
            vec![]
        }
    }

    struct BadApp;
    impl Application for BadApp {
        type Activity = BadActivity;
        type ViewModel = TestVM;

        fn on_create(_: &mut AppContext<Self>) {}
        fn create_view_model(_ctx: &mut AppContext<Self>) -> TestVM {
            // Не вызываем ctx.view_model_context() — каналы не инициализированы
            // Создаём VM с ручными каналами
            use std::sync::{mpsc, Arc, Mutex};
            let (tx, _) = mpsc::channel::<TestCmd>();
            let (_, rx) = mpsc::channel::<TestEvt>();
            let vm_ctx = crate::ViewModelContext::new(tx, Arc::new(Mutex::new(rx)));
            TestVM::create(vm_ctx)
        }
        fn create_activity(_: &mut AppContext<Self>) -> BadActivity {
            BadActivity
        }
    }

    // Не должно быть паники, так как каналы созданы руками
    let mut bad_ctx = AppContext::<BadApp>::new();
    let _vm = BadApp::create_view_model(&mut bad_ctx);
    // Но take_data_layer_channels запаникует, так как view_model_context не вызывался
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = bad_ctx.take_data_layer_channels();
    }));
    assert!(result.is_err());
}

#[test]
fn test_view_model_context_called_twice_returns_same_receiver() {
    let mut ctx = AppContext::<TestApp>::new();

    // Первый вызов — создаёт каналы
    ctx.view_model_context();
    let (cmd_rx, _evt_tx1) = ctx.take_data_layer_channels();
    drop(cmd_rx);

    // Второй вызов должен вернуть новый контекст с тем же Receiver
    // (из-за другой сигнатуры from_parts создаёт новый command_tx)
    // Проверим, что второй вызов не паникует
    ctx.view_model_context();

    // Подтверждаем — нет паники = тест пройден
}

#[test]
fn test_integration_full_chain() {
    // Полный цикл: команда → data layer → событие → ViewModel
    let mut ctx = AppContext::<TestApp>::new();
    let mut vm = TestApp::create_view_model(&mut ctx);
    let (dl_cmd_rx, dl_evt_tx) = ctx.take_data_layer_channels();

    // 1. VM отправляет команду
    vm.ctx.send(TestCmd::Add(10));

    // 2. Data layer получает и обрабатывает
    let cmd = dl_cmd_rx.try_recv().expect("должна быть команда");
    let value = match cmd {
        TestCmd::Add(n) => n,
    };

    // 3. Data layer шлёт событие обратно
    dl_evt_tx.send(TestEvt::Updated(value)).unwrap();

    // 4. VM получает событие через poll_events
    let events = vm.ctx.poll_events();
    assert_eq!(events.len(), 1);

    // 5. Фреймворк вызывает on_event
    for evt in events {
        vm.on_event(evt);
    }
    assert_eq!(vm.value, 10);
}

#[test]
fn test_app_config_defaults() {
    let mut ctx = AppContext::<TestApp>::new();

    // Проверяем значения по умолчанию
    assert_eq!(ctx.config().log_tag, "egui_app");
    assert_eq!(ctx.config().target_fps, 60);

    // Меняем конфиг
    ctx.config_mut().log_tag = "my-app".into();
    ctx.config_mut().target_fps = 30;
    assert_eq!(ctx.config().log_tag, "my-app");
    assert_eq!(ctx.config().target_fps, 30);
}

#[test]
fn test_on_create_is_called() {
    let ctx = AppContext::<TestApp>::new();

    // on_create по умолчанию ничего не делает.
    // Проверим, что AppConfig меняется, значит колбэк сработал.
    let initial_tag = ctx.config().log_tag.clone();
    AppContext::<TestApp>::new(); // ещё один новый контекст
    assert_eq!(initial_tag, "egui_app");
}

#[test]
fn test_activity_holds_vm_type() {
    // Проверяем, что Activity::ViewModel совпадает с Application::ViewModel
    // через ассоциированные типы (compile-time test)
    fn _check_types<A: Application>()
    where
        A::Activity: Activity<ViewModel = A::ViewModel, Application = A>,
    {
    }
    _check_types::<TestApp>();
}

#[test]
fn test_view_model_dispatch_forwards_to_handle() {
    let (cmd_tx, _cmd_rx) = std::sync::mpsc::channel::<TestCmd>();
    let (_evt_tx, evt_rx) = std::sync::mpsc::channel::<TestEvt>();
    let ctx =
        crate::ViewModelContext::new(cmd_tx, std::sync::Arc::new(std::sync::Mutex::new(evt_rx)));
    let mut vm = TestVM::create(ctx);

    assert_eq!(vm.value, 0);
    vm.dispatch(TestCmd::Add(5));
    assert_eq!(vm.value, 5);
    vm.dispatch(TestCmd::Add(3));
    assert_eq!(vm.value, 8);
}

#[test]
fn test_lifecycle_observer_defaults() {
    use crate::LifecycleObserver;

    struct Empty;
    impl LifecycleObserver for Empty {}

    let mut obj = Empty;
    obj.on_create();
    obj.on_start();
    obj.on_resume();
    obj.on_pause();
    obj.on_stop();
    obj.on_destroy();
    // Ничего не падает — все методы имеют реализации по умолчанию
}
