//! NestedScreen — экран с двумя уровнями вложенной навигации.
//!
//! Демонстрирует, что один экран может содержать несколько вложенных стеков,
//! каждый со своим типом сообщений и своим `ChildStack`.
//! `NavigationHost` ничего не знает про эти слои — вся навигация внутри
//! `NestedScreen` управляется через `handle_dyn()` и `handle_back()`.
//! Когда все внутренние стеки пусты — `handle_back()` возвращает `BackAction::Propagate`,
//!
//! # Слои
//!
//! - **Слой 1** (`NestedRoute`): экраны A, B, C. Тип сообщений — [`NestedMsg`].
//! - **Слой 2** (`NestedLayer2Route`): экраны X, Y. Тип сообщений — [`NestedLayer2Msg`].
//!
//! # Аргументация
//!
//! Оба типа сообщений абсолютно независимы друг от друга и от `RootMsg`.
//! `NestedScreen` пробует downcast входящего сообщения сначала в `NestedMsg`,
//! потом в `NestedLayer2Msg`. Какой подошёл — тот и обрабатывается.
//! Это позволяет добавлять новые слои без изменения существующего кода.

use egui_android_framework::core::{
    BackAction, Component as UiComponent, ComponentContext, ComponentNode, LifecycleObserver,
    PersistentState, UiWrapper,
};
use egui_android_framework::navigation::{ChildStack, ComponentFactory};
use egui_android_framework::runtime::{Dispatcher, DynDispatcher, SavedStack};
use egui_android_framework::ui::{
    containers::Column,
    modifier::{Modifier, ModifierDsl},
    theme::Theme,
    widgets::{Button, Spacer, Text, Widget},
};

use crate::navigation::{NestedLayer2Msg, NestedLayer2Route, NestedMsg, NestedRoute};
use serde::{Deserialize, Serialize};

// ─── SubScreen ─────────────────────────────────────────────────────────────

/// Подэкран слоя 1: A, B или C.
/// Содержит только заголовок и кнопку «← Назад».
#[derive(egui_android_framework::ComponentNode)]
#[component_message(NestedMsg)]
pub struct Layer1Sub {
    label: String,
}

impl Layer1Sub {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
        }
    }

    pub fn from_route(route: &NestedRoute) -> Self {
        match route {
            NestedRoute::A => Self::new("Экран A"),
            NestedRoute::B => Self::new("Экран B"),
            NestedRoute::C => Self::new("Экран C"),
        }
    }
}

impl LifecycleObserver for Layer1Sub {}

impl UiComponent for Layer1Sub {
    type State = ();
    type Message = NestedMsg;

    fn render(
        &self,
        ui: &mut UiWrapper,
        dispatch: &Dispatcher<Self::Message>,
        _ctx: &ComponentContext,
    ) {
        let c = Theme::current_from_ui(ui).colors;
        Column::new().show(ui, dispatch, |ui, dispatch| {
            Text::new(&self.label)
                .modifier(Modifier::new().padding(12.0))
                .render(ui, dispatch);
            Spacer::new(8.0).render(ui, dispatch);
            Button::new("← Назад")
                .on_click(NestedMsg::Back)
                .theme_colors(c.primary)
                .text_color(c.on_primary)
                .modifier(Modifier::new().fill_max_width().padding(8.0))
                .render(ui, dispatch);
        });
    }

    fn handle(&mut self, msg: Self::Message, ctx: &mut ComponentContext) {
        match msg {
            NestedMsg::Back => {
                self.handle_back(ctx);
            }
            _ => {}
        }
    }
    fn state(&self) -> &Self::State {
        &()
    }
}

/// Подэкран слоя 2: X или Y.
/// Содержит только заголовок и кнопку «← Назад».
#[derive(egui_android_framework::ComponentNode)]
#[component_message(NestedLayer2Msg)]
pub struct Layer2Sub {
    label: String,
}

impl Layer2Sub {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
        }
    }

    pub fn from_route(route: &NestedLayer2Route) -> Self {
        match route {
            NestedLayer2Route::X => Self::new("Экран X"),
            NestedLayer2Route::Y => Self::new("Экран Y"),
        }
    }
}

impl LifecycleObserver for Layer2Sub {}

impl UiComponent for Layer2Sub {
    type State = ();
    type Message = NestedLayer2Msg;

    fn render(
        &self,
        ui: &mut UiWrapper,
        dispatch: &Dispatcher<Self::Message>,
        _ctx: &ComponentContext,
    ) {
        let c = Theme::current_from_ui(ui).colors;
        Column::new().show(ui, dispatch, |ui, dispatch| {
            Text::new(&self.label)
                .modifier(Modifier::new().padding(12.0))
                .render(ui, dispatch);
            Spacer::new(8.0).render(ui, dispatch);
            Button::new("← Назад")
                .on_click(NestedLayer2Msg::Back)
                .theme_colors(c.primary)
                .text_color(c.on_primary)
                .modifier(Modifier::new().fill_max_width().padding(8.0))
                .render(ui, dispatch);
        });
    }

    fn handle(&mut self, msg: Self::Message, ctx: &mut ComponentContext) {
        match msg {
            NestedLayer2Msg::Back => {
                self.handle_back(ctx);
            }
            _ => {}
        }
    }
    fn state(&self) -> &Self::State {
        &()
    }
}

// ─── NestedScreen ──────────────────────────────────────────────────────────

/// Экран с двумя уровнями вложенной навигации.
///
/// Владеет двумя независимыми стеками:
/// - `stack_layer1` — ChildStack<NestedRoute> (A, B, C)
/// - `stack_layer2` — ChildStack<NestedLayer2Route> (X, Y)
///
/// Когда пользователь открывает слой 2, показываются его кнопки.
/// Платформенная Back сначала закрывает подэкран внутри слоя,
/// потом закрывает слой, потом передаётся в корневой стек.
pub struct NestedScreen {
    /// Стек слоя 1: экраны A, B, C.
    stack_layer1: ChildStack<NestedRoute>,
    /// Стек слоя 2: экраны X, Y.
    stack_layer2: ChildStack<NestedLayer2Route>,
    /// Флаг: открыт ли слой 2.
    layer2_open: bool,
}

impl NestedScreen {
    pub fn new() -> Self {
        Self {
            stack_layer1: ChildStack::new(),
            stack_layer2: ChildStack::new(),
            layer2_open: false,
        }
    }
}

impl LifecycleObserver for NestedScreen {}

// ─── Сохраняемое состояние (для рекурсивного save/restore) ──────────────

#[derive(Serialize, Deserialize)]
pub struct NestedSavedState {
    layer1: SavedStack<NestedRoute>,
    layer2: SavedStack<NestedLayer2Route>,
    layer2_open: bool,
}

impl PersistentState for NestedScreen {
    type State = NestedSavedState;

    fn save(&self) -> Self::State {
        NestedSavedState {
            layer1: self.stack_layer1.save(),
            layer2: self.stack_layer2.save(),
            layer2_open: self.layer2_open,
        }
    }

    fn restore(&mut self, state: Self::State) {
        // Фабрики для пересоздания компонентов
        struct Layer1Factory;
        impl ComponentFactory<NestedRoute> for Layer1Factory {
            fn create(&self, config: NestedRoute) -> Box<dyn ComponentNode> {
                Box::new(Layer1Sub::from_route(&config))
            }
        }
        struct Layer2Factory;
        impl ComponentFactory<NestedLayer2Route> for Layer2Factory {
            fn create(&self, config: NestedLayer2Route) -> Box<dyn ComponentNode> {
                Box::new(Layer2Sub::from_route(&config))
            }
        }

        self.stack_layer1.clear();
        self.stack_layer2.clear();

        // Пересоздаём слой 1 из сохранённых конфигураций
        self.stack_layer1
            .restore_from_saved(state.layer1, &Layer1Factory);

        // Пересоздаём слой 2 из сохранённых конфигураций
        self.stack_layer2
            .restore_from_saved(state.layer2, &Layer2Factory);

        self.layer2_open = state.layer2_open;
    }
}

impl ::egui_android_framework::core::ComponentNode for NestedScreen {
    fn render(&self, ui: &mut UiWrapper, uidynmsg_tx: &DynDispatcher, ctx: &ComponentContext) {
        // Если есть активный подэкран — показываем только его
        if let Some(active) = self.stack_layer2.active() {
            return active.render(ui, uidynmsg_tx, ctx);
        }
        if let Some(active) = self.stack_layer1.active() {
            return active.render(ui, uidynmsg_tx, ctx);
        }

        // Нет активных подэкранов — показываем меню слоя
        let dispatch1: Dispatcher<NestedMsg> = uidynmsg_tx.wrap();
        let dispatch2: Dispatcher<NestedLayer2Msg> = uidynmsg_tx.wrap();
        let c = Theme::current_from_ui(ui).colors;

        Column::new().scrollable().show(ui, &dispatch1, |ui, d1| {
            Text::new("Вложенная навигация — два уровня")
                .modifier(Modifier::new().padding(8.0))
                .render(ui, d1);
            Spacer::new(16.0).render(ui, d1);

            if self.layer2_open {
                // ─── Слой 2: X, Y ───────────────────────────────
                Text::new("─── Слой 2 ───")
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, d1);

                Button::new("Экран X")
                    .on_click(NestedLayer2Msg::Navigate(NestedLayer2Route::X))
                    .theme_colors(c.secondary)
                    .text_color(c.on_secondary)
                    .modifier(Modifier::new().fill_max_width().padding(4.0))
                    .render(ui, &dispatch2);
                Button::new("Экран Y")
                    .on_click(NestedLayer2Msg::Navigate(NestedLayer2Route::Y))
                    .theme_colors(c.secondary)
                    .text_color(c.on_secondary)
                    .modifier(Modifier::new().fill_max_width().padding(4.0))
                    .render(ui, &dispatch2);
            } else {
                // ─── Слой 1: A, B, C ────────────────────────────
                Text::new("─── Слой 1 ───")
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, d1);

                Button::new("Экран A")
                    .on_click(NestedMsg::Navigate(NestedRoute::A))
                    .theme_colors(c.secondary)
                    .text_color(c.on_secondary)
                    .modifier(Modifier::new().fill_max_width().padding(4.0))
                    .render(ui, d1);
                Button::new("Экран B")
                    .on_click(NestedMsg::Navigate(NestedRoute::B))
                    .theme_colors(c.secondary)
                    .text_color(c.on_secondary)
                    .modifier(Modifier::new().fill_max_width().padding(4.0))
                    .render(ui, d1);
                Button::new("Экран C")
                    .on_click(NestedMsg::Navigate(NestedRoute::C))
                    .theme_colors(c.secondary)
                    .text_color(c.on_secondary)
                    .modifier(Modifier::new().fill_max_width().padding(4.0))
                    .render(ui, d1);

                Spacer::new(16.0).render(ui, d1);
                Button::new("▶ Открыть слой 2")
                    .on_click(NestedMsg::OpenLayer2)
                    .theme_colors(c.primary)
                    .text_color(c.on_primary)
                    .modifier(Modifier::new().fill_max_width().padding(8.0))
                    .render(ui, d1);
            }

            Spacer::new(8.0).render(ui, d1);
            Button::new("← Назад")
                .on_click(NestedMsg::Back)
                .theme_colors(c.primary)
                .text_color(c.on_primary)
                .modifier(Modifier::new().fill_max_width().padding(8.0))
                .render(ui, d1);
        });
    }

    fn handle_dyn(&mut self, msg: Box<dyn std::any::Any + Send>, ctx: &mut ComponentContext) {
        match msg.downcast::<NestedMsg>() {
            Ok(m) => {
                log::debug!("NestedScreen: NestedMsg = {:?}", m);
                match *m {
                    NestedMsg::Navigate(r) => self
                        .stack_layer1
                        .push(r.clone(), Box::new(Layer1Sub::from_route(&r))),
                    NestedMsg::Back => {
                        self.handle_back(ctx);
                    }
                    NestedMsg::OpenLayer2 => self.layer2_open = true,
                }
            }
            Err(msg) => {
                if let Ok(m) = msg.downcast::<NestedLayer2Msg>() {
                    log::debug!("NestedScreen: NestedLayer2Msg = {:?}", m);
                    match *m {
                        NestedLayer2Msg::Navigate(r) => {
                            self.layer2_open = true;
                            self.stack_layer2
                                .push(r.clone(), Box::new(Layer2Sub::from_route(&r)));
                        }
                        NestedLayer2Msg::Back => {
                            self.stack_layer2.pop();
                            if self.stack_layer2.is_empty() {
                                self.layer2_open = false;
                            }
                        }
                    }
                } else {
                    log::error!(
                        "NestedScreen::handle_dyn: не удалось downcast — \
                         ожидался NestedMsg или NestedLayer2Msg"
                    );
                }
            }
        }
    }

    fn handle_back(&mut self, _ctx: &mut ComponentContext) -> BackAction {
        if self.layer2_open {
            if !self.stack_layer2.is_empty() {
                self.stack_layer2.pop();
                return BackAction::Handled;
            }
            self.layer2_open = false;
            return BackAction::Handled;
        }
        if !self.stack_layer1.is_empty() {
            self.stack_layer1.pop();
            return BackAction::Handled;
        }
        // Все внутренние стеки пусты — передаём родительскому стеку.
        BackAction::Propagate
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn save_state(&self) -> Option<Box<dyn std::any::Any + Send>> {
        PersistentState::save_to_boxed(self)
    }

    fn restore_state(&mut self, state: Box<dyn std::any::Any + Send>) {
        PersistentState::restore_from_boxed(self, state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_back_layer2_first() {
        let mut screen = NestedScreen::new();
        screen.layer2_open = true;
        screen
            .stack_layer2
            .push(NestedLayer2Route::X, Box::new(Layer2Sub::new("X")));
        let mut ctx = ComponentContext::new();
        assert_eq!(screen.handle_back(&mut ctx), BackAction::Handled);
        assert!(screen.stack_layer2.is_empty());
        assert!(screen.layer2_open); // слой 2 ещё «открыт» кнопками
    }

    #[test]
    fn handle_back_closes_layer2() {
        let mut screen = NestedScreen::new();
        screen.layer2_open = true;
        let mut ctx = ComponentContext::new();
        assert_eq!(screen.handle_back(&mut ctx), BackAction::Handled);
        assert!(!screen.layer2_open);
    }

    #[test]
    fn handle_back_layer1() {
        let mut screen = NestedScreen::new();
        screen
            .stack_layer1
            .push(NestedRoute::A, Box::new(Layer1Sub::new("A")));
        let mut ctx = ComponentContext::new();
        assert_eq!(screen.handle_back(&mut ctx), BackAction::Handled);
        assert!(screen.stack_layer1.is_empty());
    }

    #[test]
    fn handle_back_empty_propagates() {
        let mut screen = NestedScreen::new();
        let mut ctx = ComponentContext::new();
        assert_eq!(screen.handle_back(&mut ctx), BackAction::Propagate);
    }
}
