use std::sync::mpsc;

use crate::{Activity, ViewModel, ViewModelContext};

pub trait Application: Sized + 'static {
    type Activity: Activity<ViewModel = Self::ViewModel, Application = Self>;
    type ViewModel: ViewModel;

    fn on_create(context: &mut AppContext<Self>);
    fn create_view_model(context: &mut AppContext<Self>) -> Self::ViewModel;
    fn create_activity(context: &mut AppContext<Self>) -> Self::Activity;
}

pub struct AppContext<A: Application> {
    config: AppConfig,
    data_layer_tx: Option<mpsc::Sender<<A::ViewModel as ViewModel>::DataCommand>>,
    data_layer_rx: Option<mpsc::Receiver<<A::ViewModel as ViewModel>::Event>>,
}

impl<A: Application> AppContext<A> {
    pub fn new() -> Self {
        let (tx, _rx) = mpsc::channel::<<A::ViewModel as ViewModel>::DataCommand>();
        let (_tx, rx) = mpsc::channel::<<A::ViewModel as ViewModel>::Event>();
        Self {
            config: AppConfig::default(),
            data_layer_tx: Some(tx),
            data_layer_rx: Some(rx),
        }
    }

    pub fn take_data_layer_channels(
        &mut self,
    ) -> (
        mpsc::Sender<<A::ViewModel as ViewModel>::DataCommand>,
        mpsc::Receiver<<A::ViewModel as ViewModel>::Event>,
    ) {
        (
            self.data_layer_tx.take().expect("Channel tx already taken"),
            self.data_layer_rx.take().expect("Channel rx already taken"),
        )
    }

    pub fn view_model_context(
        &mut self,
    ) -> ViewModelContext<
        <A::ViewModel as ViewModel>::DataCommand,
        <A::ViewModel as ViewModel>::Event,
    > {
        let (tx, _rx) = mpsc::channel::<<A::ViewModel as ViewModel>::DataCommand>();
        let (_tx, rx) = mpsc::channel::<<A::ViewModel as ViewModel>::Event>();
        self.data_layer_tx = Some(tx);
        // self.data_layer_rx = Some(rx);
        ViewModelContext::new(self.data_layer_tx.as_ref().unwrap().clone(), rx)
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
