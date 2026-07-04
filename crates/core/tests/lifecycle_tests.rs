use egui_android_core::{LifecycleObserver, LifecycleState};

struct TestObserver {
    states: Vec<LifecycleState>,
}

impl LifecycleObserver for TestObserver {
    fn on_create(&mut self) {
        self.states.push(LifecycleState::Created);
    }
    fn on_start(&mut self) {
        self.states.push(LifecycleState::Started);
    }
    fn on_resume(&mut self) {
        self.states.push(LifecycleState::Resumed);
    }
    fn on_pause(&mut self) {
        self.states.push(LifecycleState::Paused);
    }
    fn on_stop(&mut self) {
        self.states.push(LifecycleState::Stopped);
    }
    fn on_destroy(&mut self) {
        self.states.push(LifecycleState::Destroyed);
    }
}

#[test]
fn test_lifecycle_order() {
    let mut observer = TestObserver { states: vec![] };
    observer.on_create();
    observer.on_start();
    observer.on_resume();
    observer.on_pause();
    observer.on_stop();
    observer.on_destroy();

    assert_eq!(
        observer.states,
        vec![
            LifecycleState::Created,
            LifecycleState::Started,
            LifecycleState::Resumed,
            LifecycleState::Paused,
            LifecycleState::Stopped,
            LifecycleState::Destroyed,
        ]
    );
}
