//! Application счётчика — корень DI.
//!
//! Использует `StateStore` для реактивного состояния.
//! `EguiRepaintSubscriber` создаётся через `init_subscriber()`.
//! Нет ручного poll() — UI обновляется реактивно.

use std::sync::mpsc;

use egui_android_framework::{
    store::StateStore, AndroidWakeHandle, AppConfig, Application, Component, EguiRepaintSubscriber,
    LifecycleObserver,
};

use crate::component::CounterComponent;
use crate::data_layer::data_layer_worker;
use crate::msg::{CounterState, Msg};

/// Приложение-счётчик.
pub struct CounterApp {
    root: CounterComponent,
    config: AppConfig,
    store: StateStore<CounterState>,
    cmd_tx: mpsc::Sender<Msg>,
    /// Подписчик живёт пока жив App (RAII).
    _subscriber: Option<EguiRepaintSubscriber>,
}

impl LifecycleObserver for CounterApp {}

impl Application for CounterApp {
    type RootComponent = CounterComponent;

    fn create() -> Self {
        let mut config = AppConfig::default();
        config.log_tag = "egui-counter2".into();

        let store = StateStore::new(CounterState { count: 0 });

        let (cmd_tx, cmd_rx) = mpsc::channel::<Msg>();
        let store_for_worker = store.clone_state();

        std::thread::spawn(move || {
            data_layer_worker(cmd_rx, store_for_worker);
        });

        let root = CounterComponent::new(store.clone_state());

        Self {
            root,
            config,
            store,
            cmd_tx,
            _subscriber: None,
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

    fn init_subscriber(&mut self, ctx: &egui::Context, wake: AndroidWakeHandle) {
        log::info!("CounterApp: создаём реактивного подписчика");
        let rx = self.store.subscribe();
        let subscriber = EguiRepaintSubscriber::new(rx, ctx.clone(), Some(wake));
        self._subscriber = Some(subscriber);
    }

    fn frame(&mut self, egui_ctx: &egui::Context, raw_input: egui::RawInput) -> egui::FullOutput {
        self.root.sync_from_store();

        let mut messages: Vec<Msg> = Vec::new();
        let full_output = egui_ctx.run(raw_input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                messages = self.root.render(ui);
            });
        });

        for msg in messages {
            self.root.handle(msg.clone());
            let _ = self.cmd_tx.send(msg);
        }

        full_output
    }
}
