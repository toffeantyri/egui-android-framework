//! ShowcaseApplication — корень DI showcase-приложения.

use std::sync::mpsc;

use egui_android_framework::{
    core::{LifecycleObserver, UiWrapper},
    runtime::AndroidWakeHandle,
    runtime::AppConfig,
    runtime::Application,
    runtime::DynDispatcher,
    runtime::StateStore,
    runtime::UiNotifier,
    ui::theme::MaterialTheme,
};

use crate::navigation_host::{NavigationHost, RootMsg};

/// Корневое состояние приложения.
#[derive(Clone, Debug, Default)]
pub struct AppState {
    pub is_dark_mode: bool,
}

/// Приложение-витрина.
pub struct ShowcaseApplication {
    root: NavigationHost,
    config: AppConfig,
    state: StateStore<AppState>,
    _notify_rx: mpsc::Receiver<()>,
    prev_dark_mode: Option<bool>,
}

impl LifecycleObserver for ShowcaseApplication {}

impl Application for ShowcaseApplication {
    type RootComponent = NavigationHost;

    fn create() -> Self {
        let config = AppConfig {
            log_tag: "egui-showcase".into(),
            target_fps: 60,
        };

        let store = StateStore::new(AppState {
            is_dark_mode: false,
        });
        let (_notify_tx, notify_rx) = mpsc::channel::<()>();

        let root = NavigationHost::new(store.clone_state());

        Self {
            root,
            config,
            state: store,
            _notify_rx: notify_rx,
            prev_dark_mode: None,
        }
    }

    fn root(&mut self) -> &mut NavigationHost {
        &mut self.root
    }

    fn root_ref(&self) -> &NavigationHost {
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
        log::info!("ShowcaseApplication: on_back_pressed (через ComponentContext)");
        self.root.on_back();
    }

    fn request_destroy(&mut self) -> bool {
        self.root.context.finish_requested
    }

    fn frame(&mut self, egui_ctx: &egui::Context, raw_input: egui::RawInput) -> egui::FullOutput {
        let app_state = self.state.state();
        let new_dark = app_state.is_dark_mode;

        // Применяем тему в зависимости от состояния
        if new_dark {
            MaterialTheme::dark().apply(egui_ctx);
        } else {
            MaterialTheme::light().apply(egui_ctx);
        }

        // При смене темы — обновляем системные бары (один раз, не каждый кадр)
        if self.prev_dark_mode != Some(new_dark) {
            self.prev_dark_mode = Some(new_dark);
            #[cfg(target_os = "android")]
            {
                use egui_android_framework::core::SystemTheme;
                use egui_android_framework::platform_android::system_bars;
                use egui_android_framework::platform_android::theme::set_clear_color_from;

                let bg_color = if new_dark {
                    MaterialTheme::dark().colors.background
                } else {
                    MaterialTheme::light().colors.background
                };

                set_clear_color_from(bg_color);
                system_bars::apply_system_bars_for_theme(if new_dark {
                    SystemTheme::Dark
                } else {
                    SystemTheme::Light
                });
            }
        }

        self.root.sync_from_store();

        let (dyn_dispatcher, dyn_receiver) = DynDispatcher::new();

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
                    NavigationHost::render_dyn(&self.root, &mut wrapper, &dyn_dispatcher);
                });
        });

        // Drain'им сообщения от View (DynDispatcher)
        for msg in dyn_receiver.try_iter() {
            if let Ok(root_msg) = msg.downcast::<RootMsg>() {
                self.root.handle_msg(*root_msg);
            }
        }

        full_output
    }
}
