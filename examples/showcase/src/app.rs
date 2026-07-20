//! ShowcaseApplication — корень DI showcase-приложения.

use std::sync::mpsc;

use egui_android_framework::{
    core::{LifecycleObserver, UiWrapper},
    platform::Waker,
    runtime::{Application, DynDispatcher, RuntimeConfig, RuntimeContext, StateStore},
    ui::theme::MaterialTheme,
};

use crate::factory::ShowcaseFactory;
use crate::navigation_host::{NavigationHost, RootMsg};

/// Корневое состояние приложения.
#[derive(Clone, Debug, Default)]
pub struct AppState {
    pub is_dark_mode: bool,
}

/// Приложение-витрина.
pub struct ShowcaseApplication {
    root: NavigationHost,
    config: RuntimeConfig,
    state: StateStore<AppState>,
    _statechanged_rx: mpsc::Receiver<()>,
    prev_dark_mode: Option<bool>,
}

impl LifecycleObserver for ShowcaseApplication {}

impl Application for ShowcaseApplication {
    type RootComponent = NavigationHost;

    fn create() -> Self {
        let config = RuntimeConfig {
            log_tag: "egui-showcase".into(),
            target_fps: 60,
        };

        let store = StateStore::new(AppState {
            is_dark_mode: false,
        });
        let (_statechanged_tx, statechanged_rx) = mpsc::channel::<()>();

        let factory = Box::new(ShowcaseFactory);
        let root = NavigationHost::new(store.clone_state(), factory);

        Self {
            root,
            config,
            state: store,
            _statechanged_rx: statechanged_rx,
            prev_dark_mode: None,
        }
    }

    fn root(&mut self) -> &mut NavigationHost {
        &mut self.root
    }

    fn root_ref(&self) -> &NavigationHost {
        &self.root
    }

    fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    fn config_mut(&mut self) -> &mut RuntimeConfig {
        &mut self.config
    }

    fn create_runtime_context(&mut self, _ctx: &egui::Context, _wake: Waker) -> RuntimeContext {
        RuntimeContext::new(None)
    }

    fn on_back_pressed(&mut self) {
        self.root.on_back();
    }

    fn request_destroy(&mut self) -> bool {
        // Завершаем приложение только если стек пуст или finish_requested выставлен
        self.root.context.finish_requested || self.root.stack.is_empty()
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

        // При смене темы — только применяем MaterialTheme в egui::Context.
        // platform-android сам определит clear_color и system_bars.
        if self.prev_dark_mode != Some(new_dark) {
            self.prev_dark_mode = Some(new_dark);
        }

        self.root.sync_from_store();

        let (uidynmsg_tx, uidynmsg_rx) = DynDispatcher::new();

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
                    NavigationHost::render_dyn(&self.root, &mut wrapper, &uidynmsg_tx);
                });
        });

        // Drain'им сообщения от View (DynDispatcher)
        //
        // Если сообщение удалось downcast'ить в RootMsg — обрабатываем
        // напрямую (навигация, тема).
        //
        // Иначе пробрасываем активному компоненту через handle_dyn() —
        // компонент сам знает свой тип сообщения (например, NestedMsg).
        // Blanket-impl ComponentNode сделает downcast и вызовет handle().
        for msg in uidynmsg_rx.try_iter() {
            // Пробуем downcast в RootMsg — сообщения корневого стека.
            // Если не получилось — пробрасываем активному компоненту
            // через handle_dyn(). Компонент сам сделает downcast в свой тип.
            match msg.downcast::<RootMsg>() {
                Ok(root_msg) => {
                    log::debug!("Получено RootMsg: {:?}", root_msg);
                    self.root.handle_msg(*root_msg);
                }
                Err(msg) => {
                    log::debug!("Получено не-RootMsg сообщение — пробрасываем в handle_dyn()");
                    if let Some(active) = self.root.stack.active_mut() {
                        active.handle_dyn(msg);
                    }
                }
            }
        }

        full_output
    }
}
