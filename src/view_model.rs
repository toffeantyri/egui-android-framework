use std::{cell::RefCell, sync::mpsc};

pub trait ViewModel: Sized + Send {
    type DataCommand: Send + 'static;
    type Event: Send + 'static;

    fn create(context: ViewModelContext<Self::DataCommand, Self::Event>) -> Self;
    fn on_event(&mut self, evt: Self::Event) {}
}

pub struct ViewModelContext<C: Send + 'static, E: Send + 'static> {
    command_tx: mpsc::Sender<C>,
    event_rx: RefCell<mpsc::Receiver<E>>,
}

impl<C: Send + 'static, E: Send + 'static> ViewModelContext<C, E> {
    pub fn new(tx: mpsc::Sender<C>, rx: mpsc::Receiver<E>) -> Self {
        Self {
            command_tx: tx,
            event_rx: RefCell::new(rx),
        }
    }

    pub fn send(&self, cmd: C) {
        let _ = self.command_tx.send(cmd);
    }

    pub fn poll_events(&self) -> Vec<E> {
        let rx = self.event_rx.borrow_mut();
        let mut events = Vec::new();
        while let Ok(evt) = rx.try_recv() {
            events.push(evt);
        }
        events
    }
}
