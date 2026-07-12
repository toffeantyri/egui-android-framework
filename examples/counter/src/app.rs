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
    core::{Component, LifecycleObserver},
    runtime::{AndroidWakeHandle, AppConfig, Application, Dispatcher, StateStore, UiNotifier},
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
    config: AppConfig,
    cmd_tx: mpsc::Sender<Msg>,
    /// Канал уведомлений от data layer.
    notify_rx: mpsc::Receiver<()>,
    /// Флаг: запрошено завершение (Back на главном экране).
    destroy_requested: bool,
    /// Состояние темы приложения.
    theme_state: AppThemeState,
}

impl LifecycleObserver for CounterApp {}

impl Application for CounterApp {
    type RootComponent = CounterComponent;

    fn create() -> Self {
        let config = AppConfig {
            log_tag: "egui-counter".into(),
            ..Default::default()
        };

        let store = StateStore::new(CounterState { count: 0 });

        let (cmd_tx, cmd_rx) = mpsc::channel::<Msg>();
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
            destroy_requested: false,
            theme_state: AppThemeState::default(),
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
        let rx = std::mem::replace(&mut self.notify_rx, mpsc::channel().1);
        Some(UiNotifier::new(ctx.clone(), Some(wake), rx))
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

        // Синхронизируем clear_color и цвета системных баров
        #[cfg(target_os = "android")]
        {
            use egui_android_framework::core::SystemTheme;
            use egui_android_framework::platform_android::system_bars;
            use egui_android_framework::platform_android::theme::set_clear_color_from;
            use egui_android_framework::ui::theme::Theme;

            let theme = Theme::current(egui_ctx);
            set_clear_color_from(theme.colors.background);

            let sys_theme = if self.theme_state.is_dark_mode {
                SystemTheme::Dark
            } else {
                SystemTheme::Light
            };
            system_bars::apply_system_bars_for_theme(sys_theme);
        }

        self.root.sync_from_store();

        let (dispatcher, receiver) = Dispatcher::new();

        let full_output = egui_ctx.run_ui(raw_input, |ctx| {
            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .fill(egui::Color32::TRANSPARENT)
                        .inner_margin(egui::Margin::ZERO)
                        .outer_margin(egui::Margin::ZERO),
                )
                .show(ctx, |ui| {
                    self.root.render(ui, &dispatcher);
                });
        });

        for msg in receiver.try_iter() {
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
                    let _ = self.cmd_tx.send(msg);
                }
            }
        }

        full_output
    }
}
