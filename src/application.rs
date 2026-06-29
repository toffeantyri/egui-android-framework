//! Новый Decompose-style Application — корень DI и владелец RootComponent.
//!
//! В отличие от старого MVVM `Application`, новый не знает про Activity
//! и ViewModel. Вместо этого он:
//!
//! - Хранит общее состояние приложения (`AppState`).
//! - Запускает data layer один раз, владеет `DataLayerHandle`.
//! - Содержит фабрику компонентов (`ComponentFactory`).
//! - Владеет RootComponent с `ChildStack`.
//!
//! # Поток данных
//!
//! ```text
//! Application::render(&mut self, ui)
//!   → root.render_current(ui) → active_component.render(ui)
//!     → active_component.handle_messages()
//!       → RootComponent обрабатывает навигационные сообщения
//!         → ChildStack::push/pop/replace
//! ```
//!
//! # Жизненный цикл
//!
//! `on_resume()` / `on_pause()` / `on_destroy()` пробрасываются
//! в RootComponent, который делегирует активному компоненту.

use crate::component::Component;
use crate::component_context::ComponentContext;
use crate::LifecycleObserver;

// ─── Новый Application ─────────────────────────────────────────────────────────

/// Decompose-style Application — корень DI.
///
/// Владеет всем деревом компонентов, data layer и общим состоянием.
/// Также реализует `LifecycleObserver` — фреймворк вызывает
/// методы жизненного цикла на самом Application, который
/// делегирует их в RootComponent и ChildStack.
pub trait Application: LifecycleObserver + Sized + 'static {
    /// Тип компонента в корне стека (обычно `Box<dyn Component>`).
    type RootComponent: Component;

    /// Создать приложение.
    fn create() -> Self;

    /// Получить мутабельную ссылку на корневой компонент.
    fn root(&mut self) -> &mut Self::RootComponent;

    /// Получить ссылку на корневой компонент.
    fn root_ref(&self) -> &Self::RootComponent;

    /// Получить конфиг приложения.
    fn config(&self) -> &AppConfig;

    /// Получить мутабельную ссылку на конфиг.
    fn config_mut(&mut self) -> &mut AppConfig;

    /// Создать инфраструктурный `UiNotifier`.
    ///
    /// Вызывается Runtime после инициализации EGL.
    /// Application передаёт `Receiver<()>` — канал уведомлений
    /// от data layer. Data layer отправляет `()` после каждого
    /// изменения состояния.
    ///
    /// Реализация по умолчанию возвращает `None`.
    fn create_notifier(
        &mut self,
        _ctx: &egui::Context,
        _wake: crate::ui_notifier::AndroidWakeHandle,
    ) -> Option<crate::ui_notifier::UiNotifier> {
        None
    }

    /// Один кадр: рендеринг компонента и обработка сообщений.
    ///
    /// Вызывается один раз за кадр из `run()`.
    /// Принимает `egui::Context` и `RawInput`, возвращает `FullOutput`
    /// для отрисовки через EGL.
    ///
    /// Реализация по умолчанию — пустой egui-кадр (заглушка).
    /// Конкретное приложение **должно** переопределить этот метод,
    /// чтобы:
    /// 1. Запустить `egui_ctx.run(raw_input, ...)`, внутри которого
    ///    вызвать `self.root().render(ui)`.
    /// 2. После `ctx.run()` обработать сообщения через `root().handle()`.
    ///
    /// См. пример `examples/counter2`.
    fn frame(&mut self, egui_ctx: &egui::Context, raw_input: egui::RawInput) -> egui::FullOutput {
        // Заглушка — пустой кадр. Переопределите в вашем Application.
        egui_ctx.run(raw_input, |_ctx| {})
    }
}

// ─── AppConfig (общий, реэкспорт из application.rs) ────────────────────────────

/// Конфигурация приложения.
#[derive(Clone)]
pub struct AppConfig {
    /// Тег для логгера Android.
    pub log_tag: String,
    /// Целевой FPS (кадров в секунду).
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

// ─── ComponentFactory ──────────────────────────────────────────────────────────

/// Фабрика компонентов — создаёт компонент по конфигурации экрана.
///
/// Каждое приложение определяет свою фабрику, которая матчит
/// `Screen` на конкретный компонент.
///
/// # Пример
///
/// ```ignore
/// fn component_factory(
///     screen: &Screen,
///     ctx: &mut ComponentContext<NavMsg, DataCmd, DataEvt>,
/// ) -> Box<dyn Component<State = …, Message = …>> {
///     match screen {
///         Screen::Login => Box::new(LoginComponent::new(ctx)),
///         Screen::Home { user_id } => Box::new(HomeComponent::new(*user_id, ctx)),
///     }
/// }
/// ```
pub type ComponentFactory<OutComp> = fn(
    config: &<OutComp as Component>::State,
    ctx: &mut ComponentContext<<OutComp as Component>::Message, (), ()>,
) -> OutComp;

// ─── DataLayerHandle (каркас) ──────────────────────────────────────────────────

/// Handle для взаимодействия с data layer.
///
/// Позволяет компонентам отправлять команды в фоновый data layer
/// через `send()`. Создаётся в `Application::create()`.
///
/// TODO: реализовать полноценный канал (Sender хранится здесь,
/// Receiver — в Application для опроса через `poll()`).
#[derive(Clone)]
pub struct DataLayerHandle {
    // TODO: добавить Sender<DataCmd>
}

impl DataLayerHandle {
    /// Создать новый handle (заглушка).
    pub fn new() -> Self {
        Self {}
    }

    /// Отправить команду в data layer (заглушка).
    pub fn send(&self, _cmd: impl Send + 'static) {
        // TODO: реализовать отправку через канал
        log::info!("DataLayerHandle::send — заглушка, команда не отправлена");
    }
}

impl Default for DataLayerHandle {
    fn default() -> Self {
        Self::new()
    }
}

// ─── AppState ──────────────────────────────────────────────────────────────────

/// Общее состояние приложения (тема, локаль, сессия пользователя).
///
/// Хранится в Application и может быть передано компонентам
/// через `ComponentContext`.
#[derive(Clone, Default)]
pub struct AppState {
    /// Тема приложения (светлая/тёмная).
    pub dark_mode: bool,
    /// Язык (локаль).
    pub locale: String,
    // TODO: добавить user session, настройки и т.д.
}

// ─── Тесты ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::Component;
    use crate::LifecycleObserver;

    /// Тестовый компонент для проверки Application.
    struct TestRoot {
        state: u32,
    }

    impl LifecycleObserver for TestRoot {}

    impl Component for TestRoot {
        type State = u32;
        type Message = ();

        fn render(&self, _ui: &mut egui::Ui) -> Vec<Self::Message> {
            vec![]
        }

        fn handle(&mut self, _msg: Self::Message) {}

        fn state(&self) -> &Self::State {
            &self.state
        }
    }

    /// Тестовая имплементация Application.
    struct TestApp {
        root: TestRoot,
        config: AppConfig,
    }

    impl LifecycleObserver for TestApp {}

    impl Application for TestApp {
        type RootComponent = TestRoot;

        fn create() -> Self {
            Self {
                root: TestRoot { state: 42 },
                config: AppConfig::default(),
            }
        }

        fn root(&mut self) -> &mut TestRoot {
            &mut self.root
        }

        fn root_ref(&self) -> &TestRoot {
            &self.root
        }

        fn config(&self) -> &AppConfig {
            &self.config
        }

        fn config_mut(&mut self) -> &mut AppConfig {
            &mut self.config
        }
    }

    #[test]
    fn test_app_creation() {
        let app = TestApp::create();
        assert_eq!(app.root_ref().state, 42);
        assert_eq!(app.config().log_tag, "egui_app");
    }

    #[test]
    fn test_app_config_customization() {
        let mut app = TestApp::create();
        app.config_mut().log_tag = "my-app".into();
        app.config_mut().target_fps = 30;
        assert_eq!(app.config().log_tag, "my-app");
        assert_eq!(app.config().target_fps, 30);
    }

    #[test]
    fn test_root_access() {
        let mut app = TestApp::create();
        let root = app.root();
        assert_eq!(root.state, 42);
    }

    #[test]
    fn test_app_state_default() {
        let state = AppState::default();
        assert!(!state.dark_mode);
        assert_eq!(state.locale, "");
    }
}
