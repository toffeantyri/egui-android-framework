use std::sync::{mpsc, Arc, Mutex};

use crate::{ViewModel, ViewModelContext};

enum TestCmd {
    Inc,
}

enum TestEvt {
    ValChanged(i32),
}

struct CounterVm {
    counter: i32,
    _ctx: ViewModelContext<TestCmd, TestEvt>,
}

impl ViewModel for CounterVm {
    type Intent = TestCmd;

    type Event = TestEvt;

    fn create(context: ViewModelContext<Self::Intent, Self::Event>) -> Self {
        Self {
            counter: 0,
            _ctx: context,
        }
    }

    fn handle(&mut self, cmd: Self::Intent) {
        match cmd {
            TestCmd::Inc => self.counter += 1,
        }
    }

    fn on_event(&mut self, evt: Self::Event) {
        let TestEvt::ValChanged(n) = evt;
        self.counter = n;
    }
}

#[test]
fn test_view_model_handle_and_dispatch() {
    let (cmd_tx, _cmd_rx) = mpsc::channel::<TestCmd>();
    let (_evt_tx, evt_rx) = mpsc::channel::<TestEvt>();
    let ctx = ViewModelContext::new(cmd_tx, Arc::new(Mutex::new(evt_rx)));
    let mut vm = CounterVm::create(ctx);

    assert_eq!(vm.counter, 0);
    vm.dispatch(TestCmd::Inc);
    assert_eq!(vm.counter, 1);
}

#[test]
fn test_view_model_event() {
    let (cmd_tx, _cmd_rx) = mpsc::channel::<TestCmd>();
    let (_evt_tx, evt_rx) = mpsc::channel::<TestEvt>();
    let ctx = ViewModelContext::new(cmd_tx, Arc::new(Mutex::new(evt_rx)));

    let mut vm = CounterVm::create(ctx);
    assert_eq!(vm.counter, 0);

    vm.on_event(TestEvt::ValChanged(12));
    assert_eq!(vm.counter, 12);
}

#[test]
fn test_view_model_send_through_context() {
    let (cmd_tx, cmd_rx) = mpsc::channel::<TestCmd>();
    let (_evt_tx, evt_rx) = mpsc::channel::<TestEvt>();
    let ctx = ViewModelContext::new(cmd_tx, Arc::new(Mutex::new(evt_rx)));

    ctx.send(TestCmd::Inc);

    // Команда должна уйти в канал
    assert!(cmd_rx.try_recv().is_ok());
}

#[test]
fn test_view_model_poll_events() {
    let (_cmd_tx, _cmd_rx) = mpsc::channel::<TestCmd>();
    let (evt_tx, evt_rx) = mpsc::channel::<TestEvt>();
    let ctx = ViewModelContext::new(_cmd_tx, Arc::new(Mutex::new(evt_rx)));

    // Событий пока нет
    assert!(ctx.poll_events().is_empty());

    // Отправляем событие через Sender (имитация data layer)
    evt_tx.send(TestEvt::ValChanged(42)).unwrap();

    let events = ctx.poll_events();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0], TestEvt::ValChanged(42)));
}
