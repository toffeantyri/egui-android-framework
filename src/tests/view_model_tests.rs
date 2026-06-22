use std::sync::mpsc::{self, Receiver, Sender};

use crate::{ViewModel, ViewModelContext};

enum TestCmd {
    Inc,
    Dec,
}

enum TestEvt {
    ValChanged(i32),
    None,
}

struct CounterVm {
    counter: i32,
    ctx: ViewModelContext<TestCmd, TestEvt>,
}

impl ViewModel for CounterVm {
    type DataCommand = TestCmd;

    type Event = TestEvt;

    fn create(context: ViewModelContext<Self::DataCommand, Self::Event>) -> Self {
        Self {
            counter: 0,
            ctx: context,
        }
    }

    fn on_event(&mut self, evt: Self::Event) {
        if let TestEvt::ValChanged(n) = evt {
            self.counter = n;
        }
    }
}

#[test]
fn test_view_model_context() {
    let (cmd_tx, _rx) = mpsc::channel::<TestCmd>();
    let (_tx, evt_rx) = mpsc::channel::<TestEvt>();
    let ctx = ViewModelContext::new(cmd_tx, evt_rx);

    ctx.send(TestCmd::Inc);

    let mut vm = CounterVm::create(ctx);
    assert_eq!(vm.counter, 0);

    vm.on_event(TestEvt::ValChanged(12));
    assert_eq!(vm.counter, 12);
}
