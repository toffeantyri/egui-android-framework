use std::sync::mpsc;

use crate::{Activity, ViewModel, ViewModelContext};

pub trait Application: Sized + 'static {
    type Activity: Activity<ViewModel = Self::ViewModel, Application = Self>;
    type ViewModel: ViewModel;

    fn on_create(context: &mut AppContext<Self>);
    fn create_view_model(context: &mut AppContext<Self>) -> Self::ViewModel;
    fn create_activity(context: &mut AppContext<Self>) -> Self::Activity;
}

/// Контекст приложения — точка сборки ViewModel, Activity и data layer.
///
/// Хранит каналы связи между ViewModel и data layer:
/// - `dl_command_rx` — data layer **читает** команды, отправленные ViewModel
/// - `dl_event_tx` — data layer **отправляет** события, читаемые ViewModel
pub struct AppContext<A: Application> {
    config: AppConfig,
    /// Data layer читает отсюда команды от ViewModel
    dl_command_rx: Option<mpsc::Receiver<<A::ViewModel as ViewModel>::DataCommand>>,
    /// Data layer отправляет сюда события для ViewModel
    dl_event_tx: Option<mpsc::Sender<<A::ViewModel as ViewModel>::Event>>,
}

impl<A: Application> Default for AppContext<A> {
    fn default() -> Self {
        Self {
            config: AppConfig::default(),
            dl_command_rx: None,
            dl_event_tx: None,
        }
    }
}

impl<A: Application> AppContext<A> {
    /// Создаёт `AppContext` с пустыми каналами.
    ///
    /// Каналы полноценно инициализируются при вызове `view_model_context()`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Забрать каналы для передачи data layer (один раз).
    ///
    /// Data layer получает:
    /// - `Receiver<DataCommand>` — для чтения команд от ViewModel
    /// - `Sender<Event>` — для отправки событий в ViewModel
    ///
    /// # Panics
    /// Паникует, если `view_model_context()` ещё не был вызван,
    /// или каналы уже были взяты.
    #[allow(clippy::type_complexity)]
    pub fn take_data_layer_channels(
        &mut self,
    ) -> (
        mpsc::Receiver<<A::ViewModel as ViewModel>::DataCommand>,
        mpsc::Sender<<A::ViewModel as ViewModel>::Event>,
    ) {
        (
            self.dl_command_rx.take().expect("take_data_layer_channels: command receiver already taken, call view_model_context() first"),
            self.dl_event_tx.take().expect("take_data_layer_channels: event sender already taken, call view_model_context() first"),
        )
    }

    /// Создать `ViewModelContext` для ViewModel (один раз).
    ///
    /// Заводит каналы и сохраняет в себе концы для data layer.
    /// ViewModel получает `Sender<DataCommand>` для отправки команд
    /// и `Receiver<Event>` для чтения событий.
    pub fn view_model_context(
        &mut self,
    ) -> ViewModelContext<
        <A::ViewModel as ViewModel>::DataCommand,
        <A::ViewModel as ViewModel>::Event,
    > {
        // Канал команд: VM (Sender) → Data layer (Receiver)
        let (vm_cmd_tx, dl_cmd_rx) = mpsc::channel::<<A::ViewModel as ViewModel>::DataCommand>();

        // Канал событий: Data layer (Sender) → VM (Receiver)
        let (dl_evt_tx, vm_evt_rx) = mpsc::channel::<<A::ViewModel as ViewModel>::Event>();

        self.dl_command_rx = Some(dl_cmd_rx);
        self.dl_event_tx = Some(dl_evt_tx);

        ViewModelContext::new(vm_cmd_tx, vm_evt_rx)
    }

    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut AppConfig {
        &mut self.config
    }
}

pub struct AppConfig {
    pub log_tag: String,
    pub target_fps: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            log_tag: "egui_app".to_owned(),
            target_fps: 60,
        }
    }
}
