#[derive(PartialEq, Debug)]
pub enum LifecycleState {
    Created,
    Started,
    Resumed,
    Paused,
    Stopped,
    Destroyed,
}

pub trait LifecycleObserver {
    fn on_create(&mut self) {}
    fn on_start(&mut self) {}
    fn on_resume(&mut self) {}
    fn on_pause(&mut self) {}
    fn on_stop(&mut self) {}
    fn on_destroy(&mut self) {}
}
