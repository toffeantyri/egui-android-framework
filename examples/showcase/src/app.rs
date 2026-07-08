//! ShowcaseApplication — корень DI showcase-приложения.

use std::sync::mpsc;

use egui_android_framework::{
    core::Component, core::LifecycleObserver, runtime::AndroidWakeHandle, runtime::AppConfig,
    runtime::Application, runtime::Dispatcher, runtime::StateStore, runtime::UiNotifier,
    ui::theme::MaterialTheme,
};

use crate::root_component::RootMsg;

use crate::root_component::RootComponent;

/// Корневое состояние приложения.
#[derive(Clone, Debug, Default)]
pub struct AppState {
    pub is_dark_mode: bool,
}

/// Приложение-витрина.
pub struct ShowcaseApplication {
    root: RootComponent,
    config: AppConfig,
    state: StateStore<AppState>,
    _notify_rx: mpsc::Receiver<()>,
    /// Dispatcher для отправки сообщений из обработчиков (например, Back).
    /// Создаётся в `create()`, Receiver drain'ится в `frame()`.
    back_dispatcher: Dispatcher<RootMsg>,
    back_receiver: Option<mpsc::Receiver<RootMsg>>,
    /// Receiver для сигнала о завершении (когда стек пуст).
    finish_rx: Option<mpsc::Receiver<()>>,
}

impl LifecycleObserver for ShowcaseApplication {}

impl Application for ShowcaseApplication {
    type RootComponent = RootComponent;

    fn create() -> Self {
        let config = AppConfig {
            log_tag: "egui-showcase".into(),
            target_fps: 60,
        };

        let store = StateStore::new(AppState {
            is_dark_mode: false,
        });
        let (_notify_tx, notify_rx) = mpsc::channel::<()>();

        let mut root = RootComponent::new(store.clone_state());

        let (back_dispatcher, back_receiver) = Dispatcher::new();
        let (finish_tx, finish_rx) = mpsc::channel();
        root.set_finish_tx(finish_tx);

        Self {
            root,
            config,
            state: store,
            _notify_rx: notify_rx,
            back_dispatcher,
            back_receiver: Some(back_receiver),
            finish_rx: Some(finish_rx),
        }
    }

    fn root(&mut self) -> &mut RootComponent {
        &mut self.root
    }

    fn root_ref(&self) -> &RootComponent {
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
        _ctx: &egui::Context,
        _wake: AndroidWakeHandle,
    ) -> Option<UiNotifier> {
        None
    }

    fn on_back_pressed(&mut self) {
        log::info!("ShowcaseApplication: on_back_pressed — отправляем RootMsg::Back");
        self.back_dispatcher.dispatch(RootMsg::Back);
    }

    fn frame(&mut self, egui_ctx: &egui::Context, raw_input: egui::RawInput) -> egui::FullOutput {
        // Применяем тему в зависимости от состояния
        let app_state = self.state.state();
        if app_state.is_dark_mode {
            MaterialTheme::dark().apply(egui_ctx);
        } else {
            MaterialTheme::light().apply(egui_ctx);
        }

        self.root.sync_from_store();

        let (dispatcher, receiver) = Dispatcher::new();

        let full_output = egui_ctx.run_ui(raw_input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                self.root.render(ui, &dispatcher);
            });
        });

        // Сначала drain'им Back-сообщения (из on_back_pressed)
        if let Some(back_rx) = &self.back_receiver {
            for msg in back_rx.try_iter() {
                self.root.handle(msg);
            }
        }

        // Потом drain'им сообщения от View
        for msg in receiver.try_iter() {
            self.root.handle(msg);
        }

        // Проверяем сигнал завершения (стек навигации пуст)
        if let Some(ref finish_rx) = self.finish_rx {
            if finish_rx.try_recv().is_ok() {
                log::info!("ShowcaseApplication: получен сигнал завершения");
                self.finish();
            }
        }

        full_output
    }
}
