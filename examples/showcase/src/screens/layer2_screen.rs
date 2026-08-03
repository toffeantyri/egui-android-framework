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
use egui_android_framework::ComponentNode;

use crate::navigation::{NestedLayer2Msg, NestedLayer2Route};
use serde::{Deserialize, Serialize};

/// Подэкран слоя 2: X или Y. Содержит только заголовок и кнопку «← Назад».
#[derive(ComponentNode)]
#[component_message(NestedLayer2Msg)]
#[back_message(NestedLayer2Msg::Back)]
#[back_handler(on_back)]
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

    /// Кастомный Back: подэкран просит закрыть себя (поп родителем).
    /// Оба варианта сообщения Back обрабатываются одинаково.
    fn on_back(&mut self, _ctx: &mut ComponentContext) -> BackAction {
        BackAction::Pop
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
    ) -> Option<BackAction> {
        // Сначала пробуем распознать собственное сообщение (меню слоя 2).
        match msg.downcast::<NestedLayer2Msg>() {
            Ok(m) => {
                log::debug!("Layer2Screen: NestedLayer2Msg = {:?}", m);
                return match *m {
                    NestedLayer2Msg::Navigate(r) => {
                        self.stack
                            .push(r.clone(), Box::new(Layer2Sub::from_route(&r)));
                        // Обработано — не поднимаем и не закрываем подэкран.
                        None
                    }
                    NestedLayer2Msg::Back => {
                        if self.stack.is_empty() {
                            // Меню без подэкранов — просим родительский стек сделать pop.
                            Some(BackAction::Propagate)
                        } else {
                            // Есть активный подэкран — обрабатываем здесь.
                            Some(self.handle_back(ctx))
                        }
                    }
                };
            }
            // Чужое сообщение — делегируем активному подэкрану.
            Err(msg) => self.stack.delegate_dyn(msg, ctx),
        }
    }

    fn handle_back(&mut self, ctx: &mut ComponentContext) -> BackAction {
        // Рекурсивно вглубь: активный подэкран обрабатывает Back первым,
        // чтобы системная Back закрывала самый глубокий экран.
        self.stack.delegate_back(ctx)
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

    /// Проверяет, что handle_dyn для NestedLayer2Msg::Back на пустом меню
    /// НЕ вызывает handle_back, а возвращает Propagate.
    #[test]
    fn handle_dyn_back_empty_menu_returns_propagate() {
        let mut screen = Layer2Screen::new();
        let mut ctx = ComponentContext::new();

        // Пустой внутренний стек — меню слоя 2.
        let action = screen.handle_dyn(Box::new(NestedLayer2Msg::Back), &mut ctx);

        assert_eq!(
            action,
            Some(BackAction::Propagate),
            "handle_dyn для Back на пустом меню должен вернуть Some(Propagate)"
        );
        assert!(screen.stack.is_empty(), "внутренний стек не изменился");
    }

    /// Проверяет, что handle_dyn для Back при наличии подэкрана
    /// вызывает handle_back (ровно 1 раз) и возвращает Some(Handled).
    #[test]
    fn handle_dyn_back_with_subscreen_calls_handle_back_once() {
        let mut screen = Layer2Screen::new();
        let mut ctx = ComponentContext::new();
        // Добавляем подэкран X
        screen
            .stack
            .push(NestedLayer2Route::X, Box::new(Layer2Sub::new("X")));
        assert_eq!(screen.stack.len(), 1);

        let action = screen.handle_dyn(Box::new(NestedLayer2Msg::Back), &mut ctx);

        assert_eq!(
            action,
            Some(BackAction::Handled),
            "handle_dyn с подэкраном должен вернуть Some(Handled) (подэкран закрыт)"
        );
        assert!(screen.stack.is_empty(), "подэкран X должен быть закрыт");
    }

    /// Полная цепочка: пустое меню → handle_dyn Back →
    /// Some(Propagate) → родитель вызовет on_back → handle_back.
    #[test]
    fn full_chain_empty_menu_back_called_once() {
        let mut screen = Layer2Screen::new();
        let mut ctx = ComponentContext::new();

        // Шаг 1: handle_dyn возвращает Some(Propagate)
        let action = screen.handle_dyn(Box::new(NestedLayer2Msg::Back), &mut ctx);
        assert_eq!(action, Some(BackAction::Propagate));
        assert!(screen.stack.is_empty());

        // Шаг 2: handle_back — единственный вызов
        let action2 = screen.handle_back(&mut ctx);
        assert_eq!(action2, BackAction::Propagate);
    }
}
