//! Пример счётчика на egui + egui-android-framework.
//!
//! Демонстрирует полную MVVM-архитектуру:
//! - ViewModel хранит состояние, владеет Sender для отправки команд в data layer
//! - Activity читает состояние из ViewModel, возвращает команды через `render()`
//! - Data layer работает в отдельном потоке, общается через mpsc-каналы
//! - Нет глобальных переменных — всё через трейты фреймворка

use egui_android_framework::{Activity, AppContext, Application, ViewModel, ViewModelContext};
use std::sync::mpsc;

#[cfg(target_os = "android")]
use egui_android_framework::android::run;

// ─── Типы команд и событий ────────────────────────────────────────────────────

/// Команда от UI к ViewModel (и далее в data layer).
#[derive(Debug)]
enum Cmd {
    Increment,
}

/// Событие от data layer к ViewModel.
#[derive(Debug)]
enum Evt {
    CountUpdated(u32),
}

// ─── Data Layer ───────────────────────────────────────────────────────────────

/// Фоновая задача: получает команды, изменяет состояние, шлёт события обратно.
fn data_layer_worker(cmd_rx: mpsc::Receiver<Cmd>, evt_tx: mpsc::Sender<Evt>) {
    let mut count = 0u32;
    loop {
        match cmd_rx.recv() {
            Ok(Cmd::Increment) => {
                count = count.wrapping_add(1);
                log::info!("DataLayer: count -> {count}");
                if evt_tx.send(Evt::CountUpdated(count)).is_err() {
                    log::info!("DataLayer: получатель событий отключён, завершаемся");
                    break;
                }
            }
            Err(_) => {
                log::info!("DataLayer: канал команд закрыт, завершаемся");
                break;
            }
        }
    }
}

// ─── ViewModel ────────────────────────────────────────────────────────────────

/// ViewModel владеет состоянием (`count`) и отправляет команды в data layer.
struct CounterViewModel {
    count: u32,
    /// Sender для отправки команд в data layer.
    cmd_tx: mpsc::Sender<Cmd>,
}

impl ViewModel for CounterViewModel {
    type DataCommand = Cmd;
    type Event = Evt;

    fn create(ctx: ViewModelContext<Cmd, Evt>) -> Self {
        let cmd_tx = ctx.command_tx().clone();
        Self { count: 0, cmd_tx }
    }

    fn handle(&mut self, cmd: Cmd) {
        match cmd {
            // Команда от UI — отправляем в data layer через ViewModelContext
            Cmd::Increment => {
                log::info!("ViewModel: handle Increment — отправляем в data layer");
                let _ = self.cmd_tx.send(Cmd::Increment);
            }
        }
    }

    fn on_event(&mut self, evt: Evt) {
        match evt {
            Evt::CountUpdated(count) => {
                log::info!("ViewModel: получено CountUpdated({count})");
                self.count = count;
            }
        }
    }
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

    fn render(
        &mut self,
        ctx: &egui::Context,
        vm: &CounterViewModel,
    ) -> Vec<<Self::ViewModel as ViewModel>::DataCommand> {
        let count = vm.count;
        let mut commands = vec![];

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
                    commands.push(Cmd::Increment);
                }
            });
        });

        commands
    }

    fn on_back_pressed(&mut self, _vm: &mut CounterViewModel) -> bool {
        log::info!("Back pressed — завершаемся");
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
        log::info!("App: createViewModel");

        // Создаём каналы
        let vm_ctx = ctx.view_model_context();

        // Забираем каналы для data layer
        let (cmd_rx, evt_tx) = ctx.take_data_layer_channels();

        // Запускаем data layer в фоне
        std::thread::spawn(move || {
            data_layer_worker(cmd_rx, evt_tx);
        });

        CounterViewModel::create(vm_ctx)
    }

    fn create_activity(_ctx: &mut AppContext<Self>) -> CounterActivity {
        CounterActivity
    }
}

// ─── Точка входа ──────────────────────────────────────────────────────────────

#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_main(app: android_activity::AndroidApp) {
    run::<CounterApp>(app);
}
