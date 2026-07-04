//! ShowcaseApplication — корень DI showcase-приложения.

use std::sync::mpsc;

use egui_android_framework::{
    core::Component, core::LifecycleObserver, runtime::AndroidWakeHandle, runtime::AppConfig,
    runtime::Application, runtime::Dispatcher, runtime::StateStore, runtime::UiNotifier,
    ui::theme::MaterialTheme,
};

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

        let root = RootComponent::new(store.clone_state());

        Self {
            root,
            config,
            state: store,
            _notify_rx: notify_rx,
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

        for msg in receiver.try_iter() {
            self.root.handle(msg);
        }

        full_output
    }
}
