use crate::{AppContext, Application, LifecycleObserver, ViewModel};

pub trait Activity: LifecycleObserver + Sized {
    type ViewModel: ViewModel;
    type Application: Application;

    fn create(context: &AppContext<Self::Application>) -> Self;
    /// Отрисовать UI и вернуть интенты для ViewModel.
    /// Фреймворк вызовет `dispatch()` для каждого интента после `render()`.
    fn render(
        &mut self,
        context: &egui::Context,
        vm: &Self::ViewModel,
    ) -> Vec<<Self::ViewModel as ViewModel>::Intent>;

    fn on_back_pressed(&mut self, _vm: &mut Self::ViewModel) -> bool {
        false
    }
    fn on_config_changed(&mut self, _context: &egui::Context) {}
    fn on_window_focus_changed(&mut self, _has_focus: bool) {}
}
