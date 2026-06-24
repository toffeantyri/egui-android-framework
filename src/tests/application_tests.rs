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
        _context: &egui::Context,
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

#[test]
fn test_data_layer_receives_commands_from_vm() {
    let mut ctx = AppContext::<TestApp>::new();
    let vm = TestApp::create_view_model(&mut ctx);

    // Data layer забирает каналы
    let (dl_cmd_rx, dl_evt_tx) = ctx.take_data_layer_channels();

    // VM отправляет команду
    vm.ctx.send(TestCmd::Add(5));

    // Data layer получает команду
    assert!(matches!(dl_cmd_rx.try_recv(), Ok(TestCmd::Add(5))));
    // Событий пока нет
    assert!(dl_evt_tx.send(TestEvt::Updated(10)).is_ok());
}

#[test]
fn test_vm_receives_events_from_data_layer() {
    let mut ctx = AppContext::<TestApp>::new();
    let mut vm = TestApp::create_view_model(&mut ctx);

    let (_dl_cmd_rx, dl_evt_tx) = ctx.take_data_layer_channels();

    // Data layer отправляет событие
    dl_evt_tx.send(TestEvt::Updated(42)).unwrap();

    // VM должна получить событие через poll_events
    let events = vm.ctx.poll_events();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0], TestEvt::Updated(42)));
    assert_eq!(vm.value, 0); // on_event ещё не вызыван

    // Фреймворк вызывает on_event для каждого события
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

    // После view_model_context каналы должны быть доступны
    let (dl_cmd_rx, dl_evt_tx) = ctx.take_data_layer_channels();
    assert!(dl_cmd_rx.try_recv().is_err()); // пусто
    assert!(dl_evt_tx.send(TestEvt::Updated(0)).is_ok());
}
