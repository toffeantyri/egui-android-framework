//! Пример счётчика на egui + egui-android-framework.
//!
//! Демонстрирует:
//! - Data Layer в отдельном потоке (mpsc-каналы)
//! - Команды ViewModel → Data Layer
//! - События Data Layer → ViewModel
//! - Обработка кнопки Back для выхода

use egui_android_framework::{
    Activity, AppContext, Application, LifecycleObserver, ViewModel, ViewModelContext,
};
use std::sync::mpsc;

#[cfg(target_os = "android")]
use egui_android_framework::android::run;

// ─── Data Layer ──────────────────────────────────────────────────────────────

/// Команды от ViewModel к Data Layer.
#[derive(Debug)]
enum Cmd {
    Increment,
}

/// События от Data Layer к ViewModel.
#[derive(Debug)]
enum Evt {
    CountUpdated(u32),
}

/// Рабочий поток Data Layer: принимает команды, обрабатывает, шлёт события.
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

struct CounterViewModel;

impl ViewModel for CounterViewModel {
    type DataCommand = Cmd;
    type Event = Evt;

    fn create(_ctx: ViewModelContext<Cmd, Evt>) -> Self {
        Self
    }

    fn handle(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Increment => {
                log::info!("ViewModel: handling Increment");
            }
        }
    }

    fn on_event(&mut self, evt: Evt) {
        match evt {
            Evt::CountUpdated(count) => {
                log::info!("ViewModel: received CountUpdated({count})");
            }
        }
    }
}

// ─── Activity ────────────────────────────────────────────────────────────────

struct CounterActivity;

impl LifecycleObserver for CounterActivity {}

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
                    ctx.data_mut(|data| data.insert_persisted(egui::Id::new("counter"), count));
                }
            });
        });
    }

    fn on_back_pressed(&mut self, _vm: &mut CounterViewModel) -> bool {
        log::info!("Back pressed — requesting shutdown");
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

        // Создаём каналы — заодно сохраняем концы для data layer в ctx.
        let vm_ctx = ctx.view_model_context();

        // Забираем каналы для data layer (Receiver команд, Sender событий).
        let (cmd_rx, evt_tx) = ctx.take_data_layer_channels();

        // Запускаем data layer в отдельном потоке.
        std::thread::spawn(move || {
            data_layer_worker(cmd_rx, evt_tx);
        });

        CounterViewModel::create(vm_ctx)
    }

    fn create_activity(_ctx: &mut AppContext<Self>) -> CounterActivity {
        CounterActivity
    }
}

// ─── Точка входа ─────────────────────────────────────────────────────────────

#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_main(app: android_activity::AndroidApp) {
    run::<CounterApp>(app);
}
