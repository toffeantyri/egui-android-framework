//! Application счётчика — корень DI.
//!
//! Использует `StateStore` для реактивного состояния.
//! Уведомления об изменении — через mpsc канал, проверяемый
//! `UiNotifier` в главном цикле. Никаких фоновых потоков.
//!
//! # BackPressed
//!
//! Системная кнопка Назад → завершение приложения.
//! `on_back_pressed()` устанавливает флаг `destroy_requested`,
//! Runtime проверяет его после `frame()` и завершает цикл.

use std::sync::mpsc;

use egui_android_framework::{
    core::{Component, LifecycleObserver, UiWrapper},
    platform::Waker,
    runtime::{Application, Dispatcher, RuntimeConfig, RuntimeContext, StateStore, UiNotifier},
    ui::theme::MaterialTheme,
};

use crate::component::CounterComponent;
use crate::data_layer::data_layer_worker;
use crate::msg::{CounterState, Msg};

/// Состояние приложения (тема).
#[derive(Clone, Debug, Default)]
pub struct AppThemeState {
    pub is_dark_mode: bool,
}

/// Приложение-счётчик.
pub struct CounterApp {
    root: CounterComponent,
    config: RuntimeConfig,
    datacmd_tx: mpsc::Sender<Msg>,
    statechanged_rx: mpsc::Receiver<()>,
    destroy_requested: bool,
    theme_state: AppThemeState,
    prev_dark_mode: Option<bool>,
}

impl LifecycleObserver for CounterApp {}

impl Application for CounterApp {
    type RootComponent = CounterComponent;

    fn create() -> Self {
        let config = RuntimeConfig {
            log_tag: "egui-counter".into(),
            ..Default::default()
        };

        let store = StateStore::new(CounterState { count: 0 });

        let (datacmd_tx, datacmd_rx) = mpsc::channel::<Msg>();
        let (statechanged_tx, statechanged_rx) = mpsc::channel::<()>();

        let store_for_worker = store.clone_state();

        std::thread::spawn(move || {
            data_layer_worker(datacmd_rx, store_for_worker, statechanged_tx);
        });

        let root = CounterComponent::new(store.clone_state());

        Self {
            root,
            config,
            datacmd_tx,
            statechanged_rx,
            destroy_requested: false,
            theme_state: AppThemeState::default(),
            prev_dark_mode: None,
        }
    }

    fn root(&mut self) -> &mut CounterComponent {
        &mut self.root
    }

    fn root_ref(&self) -> &CounterComponent {
        &self.root
    }

    fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    fn config_mut(&mut self) -> &mut RuntimeConfig {
        &mut self.config
    }

    fn create_runtime_context(&mut self, ctx: &egui::Context, wake: Waker) -> RuntimeContext {
        log::info!("CounterApp: создаём RuntimeContext");
        let rx = std::mem::replace(&mut self.statechanged_rx, mpsc::channel().1);
        let notifier = UiNotifier::new(ctx.clone(), Some(wake), rx);
        RuntimeContext::new(Some(notifier))
    }

    fn on_back_pressed(&mut self) {
        log::info!("CounterApp: Back нажата — завершаем приложение");
        self.destroy_requested = true;
    }

    fn request_destroy(&mut self) -> bool {
        self.destroy_requested
    }

    fn frame(&mut self, egui_ctx: &egui::Context, raw_input: egui::RawInput) -> egui::FullOutput {
        // Применяем тему в зависимости от состояния
        if self.theme_state.is_dark_mode {
            MaterialTheme::dark().apply(egui_ctx);
        } else {
            MaterialTheme::light().apply(egui_ctx);
        }

        // При смене темы — обновляем системные бары (один раз, не каждый кадр)
        if self.prev_dark_mode != Some(self.theme_state.is_dark_mode) {
            self.prev_dark_mode = Some(self.theme_state.is_dark_mode);
            #[cfg(target_os = "android")]
            {
                use egui_android_framework::platform_android::system_bars;
                use egui_android_framework::platform_android::theme::set_clear_color_from;
                use egui_android_framework::ui::theme::Theme;

                let theme = Theme::current(egui_ctx);
                set_clear_color_from(egui_ctx, theme.colors.background);

                system_bars::apply_system_bars_for_theme(egui_ctx);
            }
        }

        self.root.sync_from_store();

        let (uimsg_tx, uimsg_rx) = Dispatcher::new();

        let full_output = egui_ctx.run_ui(raw_input, |ctx| {
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .fill(egui::Color32::TRANSPARENT)
                        .inner_margin(egui::Margin::ZERO)
                        .outer_margin(egui::Margin::ZERO),
                )
                .show(ctx, |ui| {
                    let mut wrapper = UiWrapper::new_unconstrained(ui);
                    self.root.render(&mut wrapper, &uimsg_tx);
                });
        });

        for msg in uimsg_rx.try_iter() {
            match &msg {
                Msg::ToggleTheme => {
                    self.theme_state.is_dark_mode = !self.theme_state.is_dark_mode;
                    log::info!(
                        "CounterApp: тема переключена на {}",
                        if self.theme_state.is_dark_mode {
                            "тёмную"
                        } else {
                            "светлую"
                        }
                    );
                }
                _ => {
                    self.root.handle(msg.clone());
                    let _ = self.datacmd_tx.send(msg);
                }
            }
        }

        full_output
    }
}
