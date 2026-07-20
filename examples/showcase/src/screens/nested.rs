//! NestedScreen — экран с двумя уровнями вложенной навигации.
//!
//! Демонстрирует, что один экран может содержать несколько вложенных стеков,
//! каждый со своим типом сообщений и своим `ChildStack`.
//! `NavigationHost` ничего не знает про эти слои — вся навигация внутри
//! `NestedScreen` управляется через `handle_dyn()` и `handle_back()`.
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
    Component, ComponentNode, LifecycleObserver, PersistentState, UiWrapper,
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

impl Component for Layer1Sub {
    type State = ();
    type Message = NestedMsg;

    fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<Self::Message>) {
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

    fn handle(&mut self, _msg: Self::Message) {}
    fn state(&self) -> &Self::State {
        &()
    }
}

/// Подэкран слоя 2: X или Y.
/// Содержит только заголовок и кнопку «← Назад».
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

impl Component for Layer2Sub {
    type State = ();
    type Message = NestedLayer2Msg;

    fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<Self::Message>) {
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

    fn handle(&mut self, _msg: Self::Message) {}
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

impl ComponentNode for NestedScreen {
    fn render(&self, ui: &mut UiWrapper, uidynmsg_tx: &DynDispatcher) {
        // Если есть активный подэкран — показываем только его
        if let Some(active) = self.stack_layer2.active() {
            return active.render(ui, uidynmsg_tx);
        }
        if let Some(active) = self.stack_layer1.active() {
            return active.render(ui, uidynmsg_tx);
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

    fn handle_dyn(&mut self, msg: Box<dyn std::any::Any + Send>) {
        match msg.downcast::<NestedMsg>() {
            Ok(m) => {
                log::debug!("NestedScreen: NestedMsg = {:?}", m);
                match *m {
                    NestedMsg::Navigate(r) => self
                        .stack_layer1
                        .push(r.clone(), Box::new(Layer1Sub::from_route(&r))),
                    NestedMsg::Back => {
                        if self.layer2_open {
                            self.stack_layer2.clear();
                            self.layer2_open = false;
                        } else {
                            self.stack_layer1.pop();
                        }
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

    fn handle_back(&mut self) -> bool {
        if self.layer2_open {
            if !self.stack_layer2.is_empty() {
                self.stack_layer2.pop();
                return true;
            }
            self.layer2_open = false;
            return true;
        }
        if self.stack_layer1.is_empty() {
            return false;
        }
        self.stack_layer1.pop();
        true
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
