//! Minimal counter demo.
//!
//! State is stored directly in egui's persisted data via `ctx.data_mut()`,
//! avoiding channels, data-layer threads, and ViewModel state entirely.
//! This is the simplest possible approach to test that the rendering
//! pipeline and input handling work correctly.

use egui_android_framework::{Activity, AppContext, Application, ViewModel, ViewModelContext};

#[cfg(target_os = "android")]
use egui_android_framework::android::run;

// ─── ViewModel (empty, no state) ──────────────────────────────────────────────

struct CounterViewModel;

impl ViewModel for CounterViewModel {
    type DataCommand = ();
    type Event = ();

    fn create(_ctx: ViewModelContext<(), ()>) -> Self {
        Self
    }

    fn handle(&mut self, _cmd: ()) {}
}

// ─── Activity ─────────────────────────────────────────────────────────────────

struct CounterActivity;

impl egui_android_framework::LifecycleObserver for CounterActivity {}

impl Activity for CounterActivity {
    type ViewModel = CounterViewModel;
    type Application = CounterApp;

    fn create(_ctx: &AppContext<CounterApp>) -> Self {
        Self
    }

    fn render(&mut self, ctx: &egui::Context, _vm: &CounterViewModel) {
        let mut count =
            ctx.data_mut(|data| *data.get_persisted_mut_or(egui::Id::new("counter"), 0u32));

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(60.0);
                ui.heading("egui Counter");
                ui.add_space(16.0);
                ui.label(
                    egui::RichText::new(format!("{count}"))
                        .size(72.0)
                        .color(egui::Color32::from_rgb(66, 133, 244)),
                );
                ui.add_space(24.0);

                if ui
                    .add_sized([200.0, 56.0], egui::Button::new("+1"))
                    .clicked()
                {
                    log::info!("UI: +1 clicked");
                    count = count.wrapping_add(1);
                    ctx.data_mut(|data| {
                        data.insert_persisted(egui::Id::new("counter"), count);
                    });
                }
            });
        });
    }

    fn on_back_pressed(&mut self, _vm: &mut CounterViewModel) -> bool {
        log::info!("Back pressed — requesting shutdown");
        true
    }
}

// ─── Application ──────────────────────────────────────────────────────────────

struct CounterApp;

impl Application for CounterApp {
    type Activity = CounterActivity;
    type ViewModel = CounterViewModel;

    fn on_create(ctx: &mut AppContext<Self>) {
        ctx.config_mut().log_tag = "egui-counter".into();
    }

    fn create_view_model(ctx: &mut AppContext<Self>) -> CounterViewModel {
        // Initialize internal channels so the framework doesn't panic if
        // take_data_layer_channels() is ever called internally.
        ctx.view_model_context();
        CounterViewModel
    }

    fn create_activity(_ctx: &mut AppContext<Self>) -> CounterActivity {
        CounterActivity
    }
}

// ─── Entry Point ──────────────────────────────────────────────────────────────

#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_main(app: android_activity::AndroidApp) {
    run::<CounterApp>(app);
}
