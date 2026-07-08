//! ShowcaseApplication — корень DI showcase-приложения.

use std::sync::mpsc;

use egui_android_framework::{
    core::{BackDispatcher, Component, LifecycleObserver},
    runtime::AndroidWakeHandle,
    runtime::AppConfig,
    runtime::Application,
    runtime::Dispatcher,
    runtime::StateStore,
    runtime::UiNotifier,
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
    /// BackDispatcher — центральный обработчик системной кнопки Back.
    back_dispatcher: BackDispatcher,
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

        let (finish_tx, finish_rx) = mpsc::channel();
        root.set_finish_tx(finish_tx);

        // Создаём BackDispatcher — в него будут регистрироваться
        // обработчики компонентов (NestedScreen, диалоги).
        let back_dispatcher = BackDispatcher::new();

        Self {
            root,
            config,
            state: store,
            _notify_rx: notify_rx,
            back_dispatcher,
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
        log::info!("ShowcaseApplication: on_back_pressed");
        // BackDispatcher вызывает обработчики от высокого приоритета к низкому.
        // Если никто не обработал — RootComponent делает pop.
        let handled = self.back_dispatcher.handle();
        if !handled {
            // Никто не перехватил Back — RootComponent сам решает
            self.root.handle_back();
        }
    }

    fn finish(&mut self) {
        log::info!("ShowcaseApplication: finish — завершение приложения");
        std::process::exit(0);
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

        // Drain'им сообщения от View
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
