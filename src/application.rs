use std::sync::{mpsc, Arc, Mutex};

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
/// - `dl_command_rx` — data layer **читает** интенты, отправленные ViewModel
/// - `dl_event_tx` — data layer **отправляет** события, читаемые ViewModel
pub struct AppContext<A: Application> {
    config: AppConfig,
    /// Data layer читает отсюда интенты от ViewModel
    dl_command_rx: Option<mpsc::Receiver<<A::ViewModel as ViewModel>::Intent>>,
    /// Data layer отправляет сюда события для ViewModel
    dl_event_tx: Option<mpsc::Sender<<A::ViewModel as ViewModel>::Event>>,
    /// Shared event receiver Arc, с клонированием для передачи и ViewModel и run() одного и того же Receiver
    shared_event_rx: Option<Arc<Mutex<mpsc::Receiver<<A::ViewModel as ViewModel>::Event>>>>,
}

impl<A: Application> Default for AppContext<A> {
    fn default() -> Self {
        Self {
            config: AppConfig::default(),
            dl_command_rx: None,
            dl_event_tx: None,
            shared_event_rx: None,
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
    /// - `Receiver<Intent>` — для чтения интентов от ViewModel
    /// - `Sender<Event>` — для отправки событий в ViewModel
    ///
    /// # Panics
    /// Паникует, если `view_model_context()` ещё не был вызван,
    /// или каналы уже были взяты.
    #[allow(clippy::type_complexity)]
    pub fn take_data_layer_channels(
        &mut self,
    ) -> (
        mpsc::Receiver<<A::ViewModel as ViewModel>::Intent>,
        mpsc::Sender<<A::ViewModel as ViewModel>::Event>,
    ) {
        (
            self.dl_command_rx.take().expect("take_data_layer_channels: command receiver уже забран, сначала вызови view_model_context()"),
            self.dl_event_tx.take().expect("take_data_layer_channels: event sender уже забран, сначала вызови view_model_context()"),
        )
    }

    /// Создать (или получить) `ViewModelContext`.
    ///
    /// При первом вызове заводит mpsc-каналы и сохраняет концы для data layer.
    /// При повторных вызовах возвращает новый контекст, разделяющий тот же
    /// `Receiver` событий (через `Arc`), что и первый. `command_tx` в повторных
    /// вызовах не используется — run() только читает события.
    pub fn view_model_context(
        &mut self,
    ) -> ViewModelContext<<A::ViewModel as ViewModel>::Intent, <A::ViewModel as ViewModel>::Event>
    {
        if let Some(ref shared_rx) = self.shared_event_rx {
            return ViewModelContext::from_parts(
                mpsc::channel::<<A::ViewModel as ViewModel>::Intent>().0,
                Arc::clone(shared_rx),
            );
        }

        let (vm_cmd_tx, dl_cmd_rx) = mpsc::channel::<<A::ViewModel as ViewModel>::Intent>();
        let (dl_evt_tx, vm_evt_rx) = mpsc::channel::<<A::ViewModel as ViewModel>::Event>();

        let shared_rx = Arc::new(Mutex::new(vm_evt_rx));

        self.dl_command_rx = Some(dl_cmd_rx);
        self.dl_event_tx = Some(dl_evt_tx);
        self.shared_event_rx = Some(Arc::clone(&shared_rx));

        ViewModelContext::new(vm_cmd_tx, shared_rx)
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
