use crate::{Activity, AppContext, Application, LifecycleObserver, ViewModel};

enum DummyCmd {}
enum DummyEvt {}

struct DummyVM;

impl ViewModel for DummyVM {
    type DataCommand = DummyCmd;
    type Event = DummyEvt;

    fn create(_context: crate::ViewModelContext<Self::DataCommand, Self::Event>) -> Self {
        Self
    }

    fn handle(&mut self, _cmd: Self::DataCommand) {}
}

struct DummyApp;
impl Application for DummyApp {
    type Activity = DummyActivity;

    type ViewModel = DummyVM;

    fn on_create(_context: &mut crate::AppContext<Self>) {}

    fn create_view_model(_context: &mut crate::AppContext<Self>) -> Self::ViewModel {
        DummyVM
    }

    fn create_activity(_context: &mut crate::AppContext<Self>) -> Self::Activity {
        DummyActivity
    }
}

struct DummyActivity;

impl Activity for DummyActivity {
    type ViewModel = DummyVM;

    type Application = DummyApp;

    fn create(_context: &crate::AppContext<Self::Application>) -> Self {
        Self
    }

    fn render(&mut self, _context: &egui::Context, _vm: &Self::ViewModel) {}
}

impl LifecycleObserver for DummyActivity {}

#[test]
fn test_activity_creation() {
    let ctx = AppContext::<DummyApp>::new();
    let _activity = DummyActivity::create(&ctx);
    // если не паникует значит все Ок
}
