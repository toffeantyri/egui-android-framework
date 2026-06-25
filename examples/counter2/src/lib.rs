//! Пример счётчика на новой Decompose-style архитектуре.
//!
//! Демонстрирует:
//! - `Component` + чистая View-функция
//! - `Application2` как корень DI
//! - Data layer в фоновом потоке через mpsc
//! - `poll_data_events()` — опрос событий из data layer
//! - `run2()` вместо `run()`

use egui_android_framework::{AppConfig2, Application2, Component, LifecycleObserver};
use std::sync::mpsc;

#[cfg(target_os = "android")]
use egui_android_framework::android::run2;

// ─── Сообщения и события ───────────────────────────────────────────────────────

/// Сообщение от View к компоненту.
#[derive(Debug, Clone)]
enum Msg {
    Increment,
}

/// Событие от data layer.
#[derive(Debug)]
enum Evt {
    CountUpdated(u32),
}

// ─── Data Layer ────────────────────────────────────────────────────────────────

/// Фоновая задача: получает команды, изменяет состояние, шлёт события.
fn data_layer_worker(cmd_rx: mpsc::Receiver<Msg>, evt_tx: mpsc::Sender<Evt>) {
    let mut count = 0u32;
    loop {
        match cmd_rx.recv() {
            Ok(Msg::Increment) => {
                count = count.wrapping_add(1);
                log::info!("DataLayer: count -> {count}");
                if evt_tx.send(Evt::CountUpdated(count)).is_err() {
                    log::info!("DataLayer: получатель отключён, завершаемся");
                    break;
                }
            }
            Err(_) => {
                log::info!("DataLayer: канал закрыт, завершаемся");
                break;
            }
        }
    }
}

// ─── Component ─────────────────────────────────────────────────────────────────

/// Компонент счётчика: хранит состояние, не знает о каналах.
struct CounterComponent {
    count: u32,
}

impl LifecycleObserver for CounterComponent {}

impl Component for CounterComponent {
    type State = u32;
    type Message = Msg;

    fn render(&self, ui: &mut egui::Ui) -> Vec<Self::Message> {
        counter_view(&self.count, ui)
    }

    fn handle(&mut self, msg: Self::Message) {
        match msg {
            Msg::Increment => {
                log::info!("Component: handle Increment — отправляем в data layer");
                // Отправка происходит в CounterApp::render_and_handle,
                // так как Component не знает о cmd_tx
            }
        }
    }

    fn state(&self) -> &Self::State {
        &self.count
    }
}

/// Собственные методы CounterComponent (не из трейта Component).
impl CounterComponent {
    /// Обновить состояние из события data layer.
    fn apply_event(&mut self, evt: Evt) {
        match evt {
            Evt::CountUpdated(n) => {
                log::info!("Component: получено CountUpdated({n})");
                self.count = n;
            }
        }
    }
}

// ─── View (чистая функция) ─────────────────────────────────────────────────────

/// View-функция счётчика: читает состояние, рисует UI, возвращает сообщения.
fn counter_view(state: &u32, ui: &mut egui::Ui) -> Vec<Msg> {
    let mut messages = vec![];

    egui::CentralPanel::default().show(ui.ctx(), |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.heading("egui Counter (v2)");
            ui.add_space(16.0);
            ui.label(
                egui::RichText::new(format!("{}", state))
                    .size(72.0)
                    .color(egui::Color32::from_rgb(66, 133, 244)),
            );
            ui.add_space(24.0);

            if ui
                .add_sized([200.0, 56.0], egui::Button::new("+1"))
                .clicked()
            {
                log::info!("UI: +1 clicked");
                messages.push(Msg::Increment);
            }
        });
    });

    messages
}

// ─── Application2 ──────────────────────────────────────────────────────────────

/// Приложение-счётчик в новой архитектуре.
struct CounterApp {
    root: CounterComponent,
    config: AppConfig2,
    /// Отправитель команд в data layer (хранится здесь, а не в Component).
    cmd_tx: mpsc::Sender<Msg>,
    /// Получатель событий от data layer (опрос в render_and_handle).
    evt_rx: mpsc::Receiver<Evt>,
}

impl LifecycleObserver for CounterApp {}

impl Application2 for CounterApp {
    type RootComponent = CounterComponent;

    fn create() -> Self {
        let mut config = AppConfig2::default();
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

    fn config(&self) -> &AppConfig2 {
        &self.config
    }

    fn config_mut(&mut self) -> &mut AppConfig2 {
        &mut self.config
    }

    fn render_and_handle(
        &mut self,
        egui_ctx: &egui::Context,
        raw_input: egui::RawInput,
    ) -> (Vec<()>, egui::FullOutput) {
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

        (vec![], full_output)
    }
}

// ─── Точка входа ──────────────────────────────────────────────────────────────

#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_main(app: android_activity::AndroidApp) {
    run2::<CounterApp>(app);
}
