//! Counter example demonstrating the full ViewModel ↔ Data Layer pattern.
//!
//! The data layer runs in a background thread, owns the counter state,
//! and communicates via mpsc channels:
//!   UI → global COMMAND_TX → Cmd → data_layer_worker → Evt → ViewModel.on_event()
//!
//! Also demonstrates Back button handling.

use egui_android_framework::{Activity, AppContext, Application, ViewModel, ViewModelContext};
use std::sync::mpsc;

#[cfg(target_os = "android")]
use egui_android_framework::android::run;

/// Global sender so the UI (render) can dispatch commands to the data layer
/// without needing to mutate the ViewModel.
static COMMAND_TX: std::sync::OnceLock<mpsc::Sender<Cmd>> = std::sync::OnceLock::new();

// ─── Data Layer ──────────────────────────────────────────────────────────────

/// Commands sent from the ViewModel to the data layer.
#[derive(Debug)]
enum Cmd {
    Increment,
}

/// Events emitted by the data layer back to the ViewModel.
#[derive(Debug)]
enum Evt {
    CountUpdated(u32),
}

/// Background data-layer worker: owns the counter, processes commands,
/// sends events back to the ViewModel.
fn data_layer_worker(cmd_rx: mpsc::Receiver<Cmd>, evt_tx: mpsc::Sender<Evt>) {
    let mut count: u32 = 0;
    loop {
        match cmd_rx.recv() {
            Ok(Cmd::Increment) => {
                count = count.wrapping_add(1);
                log::info!("DataLayer: count -> {count}");
                let _ = evt_tx.send(Evt::CountUpdated(count));
            }

            Err(_) => {
                log::info!("DataLayer: channel closed, exiting");
                break;
            }
        }
    }
}

// ─── ViewModel ───────────────────────────────────────────────────────────────

struct CounterViewModel {
    count: u32,
}

impl ViewModel for CounterViewModel {
    type DataCommand = Cmd;
    type Event = Evt;

    fn create(_ctx: ViewModelContext<Cmd, Evt>) -> Self {
        Self { count: 0 }
    }

    fn handle(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Increment => {
                log::info!("ViewModel: forwarding Increment to data layer");
            }
        }
    }

    fn on_event(&mut self, evt: Evt) {
        match evt {
            Evt::CountUpdated(count) => {
                log::info!("ViewModel: received CountUpdated({count})");
                self.count = count;
            }
        }
    }
}

// ─── Activity ────────────────────────────────────────────────────────────────

struct CounterActivity;

impl egui_android_framework::LifecycleObserver for CounterActivity {}

impl Activity for CounterActivity {
    type ViewModel = CounterViewModel;
    type Application = CounterApp;

    fn create(_ctx: &AppContext<CounterApp>) -> Self {
        Self
    }

    fn render(&mut self, ctx: &egui::Context, vm: &CounterViewModel) {
        let count = vm.count;

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
                    log::info!("UI: +1 clicked, sending Cmd::Increment");
                    if let Some(tx) = COMMAND_TX.get() {
                        let _ = tx.send(Cmd::Increment);
                    }
                }
            });
        });
    }

    fn on_back_pressed(&mut self, _vm: &mut CounterViewModel) -> bool {
        log::info!("Back pressed — requesting shutdown via framework");
        true
    }
}

// ─── Application ─────────────────────────────────────────────────────────────

struct CounterApp;

impl Application for CounterApp {
    type Activity = CounterActivity;
    type ViewModel = CounterViewModel;

    fn on_create(ctx: &mut AppContext<Self>) {
        ctx.config_mut().log_tag = "egui-counter".into();
        log::info!("App: onCreate");
    }

    fn create_view_model(ctx: &mut AppContext<Self>) -> CounterViewModel {
        log::info!("App: createViewModel");

        let vm_ctx = ctx.view_model_context();

        // Take the channels for the data layer.
        let (cmd_rx, evt_tx) = ctx.take_data_layer_channels();

        // Store the command sender globally so render() can dispatch commands.
        let _ = COMMAND_TX.set(vm_ctx.command_tx().clone());

        // Spawn the data layer thread.
        std::thread::spawn(move || {
            data_layer_worker(cmd_rx, evt_tx);
        });

        CounterViewModel::create(vm_ctx)
    }

    fn create_activity(_ctx: &mut AppContext<Self>) -> CounterActivity {
        CounterActivity
    }
}

// ─── Entry Point ─────────────────────────────────────────────────────────────

#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_main(app: android_activity::AndroidApp) {
    run::<CounterApp>(app);
}
