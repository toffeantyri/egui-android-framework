//! Layer2Screen — экран-владелец слоя 2 (X, Y) вложенной навигации.
//!
//! Реализует Decompose-модель: каждый экран владеет ровно одним `ChildStack`.
//! `NestedScreen` при переходе на `NestedRoute::Layer2` разворачивает этот
//! экран, а `Layer2Screen` уже сам владеет стеком `ChildStack<NestedLayer2Route>`
//! с подэкранами X и Y. Своё состояние (`SavedStack<NestedLayer2Route>`) он
//! сохраняет/восстанавливает через `PersistentState` — слой 2 полностью
//! самодостаточен и не зависит от `NestedScreen`.

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

use crate::navigation::{NestedLayer2Msg, NestedLayer2Route};
use serde::{Deserialize, Serialize};

/// Подэкран слоя 2: X или Y. Содержит только заголовок и кнопку «← Назад».
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

    fn handle(&mut self, _msg: Self::Message, _ctx: &mut ComponentContext) {}

    fn state(&self) -> &Self::State {
        &()
    }
}

impl ComponentNode for Layer2Sub {
    fn render(&self, ui: &mut UiWrapper, dispatch: &DynDispatcher, ctx: &ComponentContext) {
        let typed = dispatch.wrap::<NestedLayer2Msg>();
        UiComponent::render(self, ui, &typed, ctx);
    }

    fn handle_dyn(
        &mut self,
        msg: Box<dyn std::any::Any + Send>,
        ctx: &mut ComponentContext,
    ) -> BackAction {
        if let Ok(typed) = msg.downcast::<NestedLayer2Msg>() {
            // Кнопка «← Назад» — подэкран просит закрыть себя (поп родителем).
            if matches!(&*typed, NestedLayer2Msg::Back) {
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

/// Экран-владелец слоя 2 (X, Y).
///
/// Владеет единственным стеком `ChildStack<NestedLayer2Route>`.
/// Подэкран X/Y — компонент [`Layer2Sub`]. Back закрывает подэкран,
/// при пустом стеке возвращает `BackAction::Propagate`.
pub struct Layer2Screen {
    /// Единственный стек слоя 2: экраны X, Y.
    pub(crate) stack: ChildStack<NestedLayer2Route>,
}

impl Layer2Screen {
    pub fn new() -> Self {
        Self {
            stack: ChildStack::new(),
        }
    }
}

impl LifecycleObserver for Layer2Screen {}

// ─── Сохраняемое состояние ───────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct Layer2SavedState {
    stack: SavedStack<NestedLayer2Route>,
}

impl PersistentState for Layer2Screen {
    type State = Layer2SavedState;

    fn save(&self) -> Self::State {
        Layer2SavedState {
            stack: self.stack.save(),
        }
    }

    fn restore(&mut self, state: Self::State) {
        struct Layer2Factory;
        impl ComponentFactory<NestedLayer2Route> for Layer2Factory {
            fn create(&self, config: NestedLayer2Route) -> Box<dyn ComponentNode> {
                Box::new(Layer2Sub::from_route(&config))
            }
        }

        self.stack.clear();
        self.stack.restore_from_saved(state.stack, &Layer2Factory);
    }
}

impl ::egui_android_framework::core::ComponentNode for Layer2Screen {
    fn render(&self, ui: &mut UiWrapper, uidynmsg_tx: &DynDispatcher, ctx: &ComponentContext) {
        // Если есть активный подэкран — показываем только его
        if let Some(active) = self.stack.active() {
            return active.render(ui, uidynmsg_tx, ctx);
        }

        // Нет активных подэкранов — меню слоя 2
        let dispatch: Dispatcher<NestedLayer2Msg> = uidynmsg_tx.wrap();
        let c = Theme::current_from_ui(ui).colors;

        Column::new()
            .scrollable()
            .show(ui, &dispatch, |ui, dispatch| {
                Text::new("Слой 2")
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);
                Spacer::new(16.0).render(ui, dispatch);

                Text::new("─── Слой 2 ───")
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);

                Button::new("Экран X")
                    .on_click(NestedLayer2Msg::Navigate(NestedLayer2Route::X))
                    .theme_colors(c.secondary)
                    .text_color(c.on_secondary)
                    .modifier(Modifier::new().fill_max_width().padding(4.0))
                    .render(ui, dispatch);
                Button::new("Экран Y")
                    .on_click(NestedLayer2Msg::Navigate(NestedLayer2Route::Y))
                    .theme_colors(c.secondary)
                    .text_color(c.on_secondary)
                    .modifier(Modifier::new().fill_max_width().padding(4.0))
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

    fn handle_dyn(
        &mut self,
        msg: Box<dyn std::any::Any + Send>,
        ctx: &mut ComponentContext,
    ) -> BackAction {
        // Сначала пробуем распознать собственное сообщение (меню слоя 2).
        match msg.downcast::<NestedLayer2Msg>() {
            Ok(m) => {
                log::debug!("Layer2Screen: NestedLayer2Msg = {:?}", m);
                return match *m {
                    NestedLayer2Msg::Navigate(r) => {
                        self.stack
                            .push(r.clone(), Box::new(Layer2Sub::from_route(&r)));
                        // Обработано — не поднимаем и не закрываем подэкран.
                        BackAction::Handled
                    }
                    NestedLayer2Msg::Back => self.handle_back(ctx),
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

    fn handle_back(&mut self, _ctx: &mut ComponentContext) -> BackAction {
        if !self.stack.is_empty() {
            self.stack.pop();
            return BackAction::Handled;
        }
        // Стек пуст — передаём родительскому стеку.
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
    fn navigate_pushes_subscreen() {
        let mut screen = Layer2Screen::new();
        let mut ctx = ComponentContext::new();
        screen.handle_dyn(
            Box::new(NestedLayer2Msg::Navigate(NestedLayer2Route::X)),
            &mut ctx,
        );
        assert_eq!(screen.stack.len(), 1);
        assert_eq!(screen.stack.active_config(), Some(&NestedLayer2Route::X));
    }

    #[test]
    fn handle_back_pops_subscreen() {
        let mut screen = Layer2Screen::new();
        screen
            .stack
            .push(NestedLayer2Route::Y, Box::new(Layer2Sub::new("Y")));
        let mut ctx = ComponentContext::new();
        assert_eq!(screen.handle_back(&mut ctx), BackAction::Handled);
        assert!(screen.stack.is_empty());
    }

    #[test]
    fn handle_back_empty_propagates() {
        let mut screen = Layer2Screen::new();
        let mut ctx = ComponentContext::new();
        assert_eq!(screen.handle_back(&mut ctx), BackAction::Propagate);
    }
}
