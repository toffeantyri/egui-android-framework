//! Application счётчика — корень DI.
//!
//! Использует `StateStore` для реактивного состояния.
//! Уведомления об изменении — через mpsc канал, проверяемый
//! `UiNotifier` в главном цикле. Никаких фоновых потоков.

use std::sync::mpsc;

use egui_android_framework::{
    store::StateStore, AndroidWakeHandle, AppConfig, Application, Component, Dispatcher,
    LifecycleObserver, UiNotifier,
};

use crate::component::CounterComponent;
use crate::data_layer::data_layer_worker;
use crate::msg::{CounterState, Msg};

/// Приложение-счётчик.
pub struct CounterApp {
    root: CounterComponent,
    config: AppConfig,
    cmd_tx: mpsc::Sender<Msg>,
    /// Канал уведомлений от data layer.
    notify_rx: mpsc::Receiver<()>,
}

impl LifecycleObserver for CounterApp {}

impl Application for CounterApp {
    type RootComponent = CounterComponent;

    fn create() -> Self {
        let mut config = AppConfig::default();
        config.log_tag = "egui-counter2".into();

        let store = StateStore::new(CounterState { count: 0 });

        let (cmd_tx, cmd_rx) = mpsc::channel::<Msg>();
        // Канал уведомлений: data layer → Runtime
        let (notify_tx, notify_rx) = mpsc::channel::<()>();

        let store_for_worker = store.clone_state();

        std::thread::spawn(move || {
            data_layer_worker(cmd_rx, store_for_worker, notify_tx);
        });

        let root = CounterComponent::new(store.clone_state());

        Self {
            root,
            config,
            cmd_tx,
            notify_rx,
        }
    }

    fn root(&mut self) -> &mut CounterComponent {
        &mut self.root
    }

    fn root_ref(&self) -> &CounterComponent {
        &self.root
    }

    fn config(&self) -> &AppConfig {
        &self.config
    }

    fn config_mut(&mut self) -> &mut AppConfig {
        &mut self.config
    }

    fn create_notifier(
        &mut self,
        ctx: &egui::Context,
        wake: AndroidWakeHandle,
    ) -> Option<UiNotifier> {
        log::info!("CounterApp: создаём UiNotifier");
        // Передаём notify_rx в UiNotifier — он будет проверять
        // его в главном цикле без блокировок.
        let rx = std::mem::replace(&mut self.notify_rx, mpsc::channel().1);
        Some(UiNotifier::new(ctx.clone(), Some(wake), rx))
    }

    fn frame(&mut self, egui_ctx: &egui::Context, raw_input: egui::RawInput) -> egui::FullOutput {
        self.root.sync_from_store();

        let (dispatcher, receiver) = Dispatcher::new();

        let full_output = egui_ctx.run(raw_input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                self.root.render(ui, &dispatcher);
            });
        });

        for msg in receiver.try_iter() {
            self.root.handle(msg.clone());
            let _ = self.cmd_tx.send(msg);
        }

        full_output
    }
}
