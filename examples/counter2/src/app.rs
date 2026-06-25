//! Application счётчика — корень DI.

use std::sync::mpsc;

use egui_android_framework::{AppConfig, Application, Component, LifecycleObserver};

use crate::component::CounterComponent;
use crate::data_layer::data_layer_worker;
use crate::msg::{Evt, Msg};

/// Приложение-счётчик.
pub struct CounterApp {
    root: CounterComponent,
    config: AppConfig,
    cmd_tx: mpsc::Sender<Msg>,
    evt_rx: mpsc::Receiver<Evt>,
}

impl LifecycleObserver for CounterApp {}

impl Application for CounterApp {
    type RootComponent = CounterComponent;

    fn create() -> Self {
        let mut config = AppConfig::default();
        config.log_tag = "egui-counter2".into();

        // Создаём каналы для data layer
        let (cmd_tx, cmd_rx) = mpsc::channel::<Msg>();
        let (evt_tx, evt_rx) = mpsc::channel::<Evt>();

        // Запускаем data layer в фоне
        std::thread::spawn(move || {
            data_layer_worker(cmd_rx, evt_tx);
        });

        Self {
            root: CounterComponent { count: 0 },
            config,
            cmd_tx,
            evt_rx,
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
        // 1. Опрашиваем события от data layer, обновляем состояние компонента
        while let Ok(evt) = self.evt_rx.try_recv() {
            self.root.apply_event(evt);
        }

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
