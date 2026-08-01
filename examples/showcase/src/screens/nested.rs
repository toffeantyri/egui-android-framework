//! NestedScreen — экран с двухуровневой вложенной навигацией (Decompose-style).
//!
//! Владеет ОДНИМ стеком `ChildStack<NestedRoute>` (A, B, C). Переход на
//! `NestedRoute::Layer2` разворачивается в самостоятельный экран
//! [`super::layer2_screen::Layer2Screen`], который владеет собственным
//! стеком `NestedLayer2Route` (X, Y) и обрабатывает свой тип сообщений
//! `NestedLayer2Msg`. `NavigationHost` ничего не знает про эти слои —
//! вся навигация внутри управляется через `handle_dyn()` и `handle_back()`.
//!
//! Когда стек `NestedScreen` пуст — `handle_back()` возвращает `BackAction::Propagate`.

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

use crate::navigation::{NestedMsg, NestedRoute};
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
            // Слой 2 создаётся отдельным экраном Layer2Screen, а не Layer1Sub.
            NestedRoute::Layer2 => unreachable!("Layer2 разворачивается в Layer2Screen"),
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

    fn handle(&mut self, _msg: Self::Message, _ctx: &mut ComponentContext) {}
    fn state(&self) -> &Self::State {
        &()
    }
}

impl ComponentNode for Layer1Sub {
    fn render(&self, ui: &mut UiWrapper, dispatch: &DynDispatcher, ctx: &ComponentContext) {
        let typed = dispatch.wrap::<NestedMsg>();
        UiComponent::render(self, ui, &typed, ctx);
    }

    fn handle_dyn(
        &mut self,
        msg: Box<dyn std::any::Any + Send>,
        ctx: &mut ComponentContext,
    ) -> BackAction {
        if let Ok(typed) = msg.downcast::<NestedMsg>() {
            // Кнопка «← Назад» — подэкран просит закрыть себя (поп родителем).
            if matches!(&*typed, NestedMsg::Back) {
                return BackAction::Pop;
            }
            UiComponent::handle(self, *typed, ctx);
        }
        BackAction::Propagate
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Экран с двухуровневой вложенной навигацией (Decompose-style).
///
/// Владеет ОДНИМ стеком `ChildStack<NestedRoute>` (A, B, C). Переход на
/// `NestedRoute::Layer2` разворачивается в самостоятельный экран
/// `Layer2Screen`, который владеет собственным стеком `NestedLayer2Route`
/// (X, Y). Фабрика `NestedRoute -> Box<dyn ComponentNode>` создаёт нужный
/// подэкран для каждого маршрута.
pub struct NestedScreen {
    /// Единственный стек слоя 1: экраны A, B, C, Layer2.
    stack: ChildStack<NestedRoute>,
}

impl NestedScreen {
    pub fn new() -> Self {
        Self {
            stack: ChildStack::new(),
        }
    }
}

impl LifecycleObserver for NestedScreen {}

// ─── Сохраняемое состояние (для рекурсивного save/restore) ──────────────

#[derive(Serialize, Deserialize)]
pub struct NestedSavedState {
    stack: SavedStack<NestedRoute>,
}

impl PersistentState for NestedScreen {
    type State = NestedSavedState;

    fn save(&self) -> Self::State {
        NestedSavedState {
            stack: self.stack.save(),
        }
    }

    fn restore(&mut self, state: Self::State) {
        // Фабрика пересоздаёт нужный подэкран для каждого маршрута:
        // A/B/C -> Layer1Sub, Layer2 -> Layer2Screen.
        struct NestedScreenFactory;
        impl ComponentFactory<NestedRoute> for NestedScreenFactory {
            fn create(&self, config: NestedRoute) -> Box<dyn ComponentNode> {
                match config {
                    NestedRoute::Layer2 => {
                        Box::new(crate::screens::layer2_screen::Layer2Screen::new())
                    }
                    r @ (NestedRoute::A | NestedRoute::B | NestedRoute::C) => {
                        Box::new(Layer1Sub::from_route(&r))
                    }
                }
            }
        }

        self.stack.clear();
        self.stack
            .restore_from_saved(state.stack, &NestedScreenFactory);
    }
}

impl ::egui_android_framework::core::ComponentNode for NestedScreen {
    fn render(&self, ui: &mut UiWrapper, uidynmsg_tx: &DynDispatcher, ctx: &ComponentContext) {
        // Если есть активный подэкран — показываем только его
        if let Some(active) = self.stack.active() {
            return active.render(ui, uidynmsg_tx, ctx);
        }

        // Нет активных подэкранов — показываем меню слоя 1 и кнопку «Слой 2»
        let dispatch1: Dispatcher<NestedMsg> = uidynmsg_tx.wrap();
        let c = Theme::current_from_ui(ui).colors;

        Column::new().scrollable().show(ui, &dispatch1, |ui, d1| {
            Text::new("Вложенная навигация — два уровня")
                .modifier(Modifier::new().padding(8.0))
                .render(ui, d1);
            Spacer::new(16.0).render(ui, d1);

            // ─── Слой 1: A, B, C, Layer2 ───────────────────────
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
            Button::new("▶ Слой 2 (X, Y)")
                .on_click(NestedMsg::Navigate(NestedRoute::Layer2))
                .theme_colors(c.primary)
                .text_color(c.on_primary)
                .modifier(Modifier::new().fill_max_width().padding(8.0))
                .render(ui, d1);

            Spacer::new(8.0).render(ui, d1);
            Button::new("← Назад")
                .on_click(NestedMsg::Back)
                .theme_colors(c.primary)
                .text_color(c.on_primary)
                .modifier(Modifier::new().fill_max_width().padding(8.0))
                .render(ui, d1);
        });
    }

    fn handle_dyn(
        &mut self,
        msg: Box<dyn std::any::Any + Send>,
        ctx: &mut ComponentContext,
    ) -> BackAction {
        // Сначала пробуем распознать собственное сообщение (меню слоя 1).
        match msg.downcast::<NestedMsg>() {
            Ok(m) => {
                log::debug!("NestedScreen: NestedMsg = {:?}", m);
                return match *m {
                    NestedMsg::Navigate(r) => {
                        let component: Box<dyn ComponentNode> = match &r {
                            NestedRoute::Layer2 => {
                                Box::new(crate::screens::layer2_screen::Layer2Screen::new())
                            }
                            r => Box::new(Layer1Sub::from_route(r)),
                        };
                        self.stack.push(r.clone(), component);
                        // Обработано — не поднимаем и не закрываем подэкран.
                        BackAction::Handled
                    }
                    NestedMsg::Back => self.handle_back(ctx),
                };
            }
            // Чужое сообщение — делегируем активному подэкрану.
            Err(msg) => {
                if let Some(active) = self.stack.active_mut() {
                    return match active.handle_dyn(msg, ctx) {
                        // Подэкран попросил закрыть себя (кнопка «← Назад»)
                        // или не обработал (Propagate) — закрываем активный подэкран.
                        BackAction::Pop | BackAction::Propagate => {
                            self.stack.pop();
                            BackAction::Handled
                        }
                        BackAction::Handled => BackAction::Handled,
                        BackAction::Finish => BackAction::Finish,
                    };
                }
                BackAction::Propagate
            }
        }
    }

    fn handle_back(&mut self, ctx: &mut ComponentContext) -> BackAction {
        // Рекурсивно вглубь: активный подэкран обрабатывает Back первым,
        // чтобы системная Back закрывала самый глубокий экран (X), а не прыгала
        // через несколько уровней сразу.
        if let Some(active) = self.stack.active_mut() {
            return match active.handle_back(ctx) {
                // Подэкран просит закрыть себя или не обработал — закрываем его.
                BackAction::Pop | BackAction::Propagate => {
                    self.stack.pop();
                    BackAction::Handled
                }
                BackAction::Handled => BackAction::Handled,
                BackAction::Finish => BackAction::Finish,
            };
        }
        // Внутренний стек пуст — передаём родительскому стеку.
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
    use crate::navigation::{NestedLayer2Msg, NestedLayer2Route};

    #[test]
    fn navigate_layer2_pushes_layer2_screen() {
        let mut screen = NestedScreen::new();
        let mut ctx = ComponentContext::new();
        screen.handle_dyn(Box::new(NestedMsg::Navigate(NestedRoute::Layer2)), &mut ctx);
        assert_eq!(screen.stack.len(), 1, "в стеке должен быть слой 2");
        assert_eq!(screen.stack.active_config(), Some(&NestedRoute::Layer2));
    }

    #[test]
    fn handle_dyn_delegates_to_active_subscreen() {
        // Открываем слой 2 (в стеке активен Layer2Screen).
        let mut screen = NestedScreen::new();
        let mut ctx = ComponentContext::new();
        screen.handle_dyn(Box::new(NestedMsg::Navigate(NestedRoute::Layer2)), &mut ctx);
        assert_eq!(screen.stack.len(), 1);

        // Сообщение от кнопки X на экране Слой 2 должно дойти до Layer2Screen
        // и развернуть подэкран X.
        screen.handle_dyn(
            Box::new(NestedLayer2Msg::Navigate(NestedLayer2Route::X)),
            &mut ctx,
        );

        let layer2 = screen.stack.active().unwrap();
        let layer2 = layer2
            .as_any()
            .downcast_ref::<crate::screens::layer2_screen::Layer2Screen>()
            .unwrap();
        assert_eq!(layer2.stack.len(), 1, "слой 2 должен открыть подэкран X");
    }

    #[test]
    fn handle_back_pops_subscreen() {
        let mut screen = NestedScreen::new();
        screen
            .stack
            .push(NestedRoute::A, Box::new(Layer1Sub::new("A")));
        let mut ctx = ComponentContext::new();
        assert_eq!(screen.handle_back(&mut ctx), BackAction::Handled);
        assert!(screen.stack.is_empty());
    }

    #[test]
    fn handle_back_empty_propagates() {
        let mut screen = NestedScreen::new();
        let mut ctx = ComponentContext::new();
        assert_eq!(screen.handle_back(&mut ctx), BackAction::Propagate);
    }

    #[test]
    fn back_button_on_subscreen_closes_only_that_subscreen() {
        // Nested -> Layer2 (активен Layer2Screen) -> открыт подэкран X.
        let mut screen = NestedScreen::new();
        let mut ctx = ComponentContext::new();
        screen.handle_dyn(Box::new(NestedMsg::Navigate(NestedRoute::Layer2)), &mut ctx);
        screen.handle_dyn(
            Box::new(NestedLayer2Msg::Navigate(NestedLayer2Route::X)),
            &mut ctx,
        );

        // Кнопка «← Назад» на подэкране X шлёт NestedLayer2Msg::Back.
        let action = screen.handle_dyn(Box::new(NestedLayer2Msg::Back), &mut ctx);
        assert_eq!(action, BackAction::Handled, "подэкран закрыт пойман");

        // Подэкран X закрылся, но Layer2Screen и NestedScreen остались.
        let layer2 = screen.stack.active().unwrap();
        let layer2 = layer2
            .as_any()
            .downcast_ref::<crate::screens::layer2_screen::Layer2Screen>()
            .unwrap();
        assert_eq!(layer2.stack.len(), 0, "X закрыт внутри слоя 2");
        assert_eq!(screen.stack.len(), 1, "Layer2 остался в NestedScreen");
    }

    #[test]
    fn platform_back_on_subscreen_closes_only_that_subscreen() {
        let mut screen = NestedScreen::new();
        let mut ctx = ComponentContext::new();
        screen.handle_dyn(Box::new(NestedMsg::Navigate(NestedRoute::Layer2)), &mut ctx);
        screen.handle_dyn(
            Box::new(NestedLayer2Msg::Navigate(NestedLayer2Route::X)),
            &mut ctx,
        );

        let action = screen.handle_back(&mut ctx);
        assert_eq!(action, BackAction::Handled);

        let layer2 = screen.stack.active().unwrap();
        let layer2 = layer2
            .as_any()
            .downcast_ref::<crate::screens::layer2_screen::Layer2Screen>()
            .unwrap();
        assert_eq!(layer2.stack.len(), 0, "X закрыт слоем 2");
        assert_eq!(screen.stack.len(), 1, "Layer2 остался");
    }
}
