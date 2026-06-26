//! Application счётчика — корень DI.
//!
//! Использует `StateStore` для реактивного состояния.
//! Больше не нужен ручной `poll()` — `EguiRepaintSubscriber`
//! автоматически вызывает `request_repaint()` и `wake()` при изменении.

use std::sync::mpsc;

use egui_android_framework::{
    egui_subscriber::EguiRepaintSubscriber, store::StateStore, AppConfig, Application, Component,
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
    /// Подписчик живет пока жив App (RAII).
    _subscriber: Option<EguiRepaintSubscriber>,
}

impl LifecycleObserver for CounterApp {}

impl Application for CounterApp {
    type RootComponent = CounterComponent;

    fn create() -> Self {
        let mut config = AppConfig::default();
        config.log_tag = "egui-counter2".into();

        // Создаём реактивное состояние
        let store = StateStore::new(CounterState { count: 0 });

        // Создаём каналы для data layer (старый mpsc для фонового потока)
        let (cmd_tx, cmd_rx) = mpsc::channel::<Msg>();
        let store_for_worker = store.clone_state();

        // Запускаем data layer в фоне — он теперь пишет в StateStore
        std::thread::spawn(move || {
            data_layer_worker(cmd_rx, store_for_worker);
        });

        // Подписчик будет создан позже, когда появится egui::Context и waker.
        // Пока оставляем None — run.rs сам создаст подписку.
        Self {
            root: CounterComponent { count: 0 },
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

    fn frame(&mut self, egui_ctx: &egui::Context, raw_input: egui::RawInput) -> egui::FullOutput {
        // 1. Получаем snapshot состояния из store (реактивно, без poll)
        let _state = self.store.state();

        // 2. Запускаем egui-кадр, внутри рендерим компонент
        let mut messages: Vec<Msg> = Vec::new();
        let full_output = egui_ctx.run(raw_input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                messages = self.root.render(ui);
            });
        });

        // 3. Обрабатываем сообщения от компонента — отправляем в data layer
        for msg in messages {
            self.root.handle(msg.clone());
            let _ = self.cmd_tx.send(msg);
        }

        full_output
    }
}
