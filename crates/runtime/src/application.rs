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
//! Application::frame(&mut self, egui_ctx, raw_input)
//!   → sync_from_store()
//!     → Dispatcher::new()
//!       → root.render(ui, &dispatcher) — View диспатчит сообщения в момент события
//!         → drain receiver → handle(msg) → cmd_tx.send(msg)
//! ```
//!
//! # Жизненный цикл
//!
//! `on_resume()` / `on_pause()` / `on_destroy()` пробрасываются
//! в RootComponent, который делегирует активному компоненту.

use crate::ui_notifier::{AndroidWakeHandle, UiNotifier};

use egui;

// ─── Новый Application ─────────────────────────────────────────────────────────

/// Decompose-style Application — корень DI.
///
/// Владеет всем деревом компонентов, data layer и общим состоянием.
/// Также реализует `LifecycleObserver` — фреймворк вызывает
/// методы жизненного цикла на самом Application, который
/// делегирует их в RootComponent и ChildStack.
///
/// Примечание: `LifecycleObserver` определён в `egui-android-core`.
/// Application будет наследовать его после переноса core.
/// Пока методы lifecycle объявлены прямо здесь.
pub trait Application: Sized + 'static {
    /// Тип компонента в корне стека (обычно `Box<dyn Component>`).
    /// Пока не привязан к Component из core — будет позже.
    type RootComponent: ?Sized;

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
        _wake: AndroidWakeHandle,
    ) -> Option<UiNotifier> {
        None
    }

    /// Один кадр: рендеринг компонента и обработка сообщений.
    ///
    /// Вызывается один раз за кадр из `run()`.
    fn frame(&mut self, egui_ctx: &egui::Context, raw_input: egui::RawInput) -> egui::FullOutput {
        egui_ctx.run_ui(raw_input, |_ctx| {})
    }

    /// Обработать нажатие системной кнопки Back.
    ///
    /// Вызывается из platform-android при перехвате AKEYCODE_BACK.
    /// Platform не знает про Msg — Application сам решает, какое сообщение
    /// диспатчить в корневой компонент (например, `RootMsg::Back`).
    ///
    /// Реализация по умолчанию — заглушка (не делает ничего).
    /// Приложение **должно** переопределить, если использует ChildStack.
    fn on_back_pressed(&mut self) {}

    /// Запросить завершение приложения.
    ///
    /// Вызывается когда стек навигации пуст
    /// (пользователь нажал Back на главном экране).
    ///
    /// Устанавливает флаг destroy_requested = true.
    /// Runtime (run.rs) должен проверить этот флаг после frame().
    fn request_destroy(&mut self) -> bool {
        false
    }

    // ─── Методы жизненного цикла ─────────────────────────────────────────

    /// Компонент создан.
    fn on_create(&mut self) {}
    /// Компонент стал видим.
    fn on_start(&mut self) {}
    /// Компонент стал активным (на вершине стека).
    fn on_resume(&mut self) {}
    /// Компонент приостановлен (с вершины стека убран другой компонент).
    fn on_pause(&mut self) {}
    /// Компонент остановлен (полностью невидим).
    fn on_stop(&mut self) {}
    /// Компонент уничтожен.
    fn on_destroy(&mut self) {}
}

// ─── AppConfig ─────────────────────────────────────────────────────────────────

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

// ─── DataLayerHandle (каркас) ──────────────────────────────────────────────────

/// Handle для взаимодействия с data layer.
///
/// Позволяет компонентам отправлять команды в фоновый data layer
/// через `send()`. Создаётся в `Application::create()`.
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
