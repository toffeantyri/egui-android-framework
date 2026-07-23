//! NavigationHost — хост навигации с ChildStack.
//!
//! Тонкая обёртка над [`ChildStack`] с Back-логикой и save/restore.
//! Не реализует Component трейт — рендер через render_dyn.
//!
//! # Обработка Back (Decompose-style)
//!
//! 1. `stack.on_back()` — цепочка внутри ChildStack:
//!    - `active.handle_back()` — компонент перехватывает (NestedScreen, BackCustomScreen)
//!    - `pop()` — стандартное поведение
//! 2. Если `on_back()` вернул `false` — стек пуст или Home → `finish_requested = true`

use egui_android_framework::{
    core::{ComponentContext, LifecycleObserver, UiWrapper},
    navigation::{ChildStack, ComponentFactory},
    runtime::{DynDispatcher, SavedStack, StateStore},
};

use crate::app::AppState;
use crate::navigation::Route;
use crate::screens::themes::ThemesScreen;

/// Сообщения навигации.
#[derive(Clone, Debug)]
pub enum RootMsg {
    /// Перейти по маршруту в корневой стек.
    Navigate(Route),
    /// Вернуться назад (pop из стека).
    Back,
    /// Переключить тему.
    ToggleTheme,
}

/// Хост навигации — координатор между Application и экранами.
///
/// Владеет корневым ChildStack, ComponentContext, Store.
/// Не реализует Component — рендер через render_dyn().
pub struct NavigationHost {
    /// Корневой стек навигации.
    pub stack: ChildStack<Route>,
    store: StateStore<AppState>,
    /// Контекст (BackDispatcher + finish_requested).
    pub context: ComponentContext,
    /// Фабрика компонентов (создание экранов по маршруту).
    factory: Box<dyn ComponentFactory<Route>>,
}

impl NavigationHost {
    /// Создать NavigationHost.
    pub fn new(store: StateStore<AppState>, factory: Box<dyn ComponentFactory<Route>>) -> Self {
        let mut stack = ChildStack::new();
        let ctx = ComponentContext::new();

        // Добавляем Home в корневой стек
        stack.push(Route::Home, factory.create(Route::Home));

        Self {
            stack,
            store,
            context: ctx,
            factory,
        }
    }

    pub fn sync_from_store(&mut self) {
        let app_state = self.store.state();
        // Пробрасываем is_dark_mode в ThemesScreen
        if let Some(active) = self.stack.active_mut() {
            if let Some(themes) = active.as_any_mut().downcast_mut::<ThemesScreen>() {
                themes.is_dark_mode = app_state.is_dark_mode;
            }
        }
    }

    /// Обработать Back.
    ///
    /// Делегирует `ChildStack::on_back()`.
    /// Если Back не обработан — завершение приложения.
    pub fn on_back(&mut self) {
        if !self.stack.on_back() {
            self.context.finish_requested = true;
        }
    }

    /// Обработать сообщение от UI.
    pub fn handle_msg(&mut self, msg: RootMsg) {
        match msg {
            RootMsg::Navigate(route) => {
                self.stack.push(route.clone(), self.factory.create(route));
            }
            RootMsg::Back => {
                self.on_back();
            }
            RootMsg::ToggleTheme => {
                self.store.update(|s| s.is_dark_mode = !s.is_dark_mode);
            }
        }
    }

    /// Рендеринг с DynDispatcher.
    pub fn render_dyn(&self, ui: &mut UiWrapper, uidynmsg_tx: &DynDispatcher) {
        if let Some(active) = self.stack.active() {
            active.render(ui, uidynmsg_tx);
        }
    }

    // --- Сохранение/восстановление (Decompose-style) ---

    /// Сохранить состояние стека навигации.
    pub fn save(&self) -> SavedStack<Route> {
        self.stack.save()
    }

    /// Восстановить состояние навигации из сохранённого.
    pub fn restore(&mut self, saved: SavedStack<Route>) {
        log::info!(
            "NavigationHost::restore: восстанавливаем {} элементов",
            saved.items.len()
        );
        self.stack.restore_from_saved(saved, &*self.factory);
    }
}

impl LifecycleObserver for NavigationHost {}
