//! StateScreen — демонстрация сохраняемого состояния через PersistentState.
//!
//! Показывает разницу между:
//! - **PersistentState** (бизнес-данные) — сохраняется при повороте экрана
//! - **remember** (UI-состояние) — сбрасывается при повороте
//!
//! Аналог Decompose: `stateKeeper` для данных, `remember` для UI.

pub use egui_android_framework::core;
use egui_android_framework::core::{
    BackAction, Component, ComponentContext, ComponentNode, LifecycleObserver, UiWrapper,
};
use egui_android_framework::runtime::Dispatcher;
use egui_android_framework::ui::{
    containers::Column,
    modifier::{Modifier, ModifierDsl},
    remember,
    theme::Theme,
    widgets::{Button, Spacer, Text, Widget},
};
use egui_android_framework::Component;

/// Сообщения экрана состояния.
#[derive(Clone, Debug)]
pub enum StateScreenMsg {
    Increment,
    Decrement,
    Reset,
    Back,
}

#[derive(Component)]
#[persistent_fields(counter)]
pub struct StateScreen {
    counter: i32,
    // Поле back_requested не нужно — флаг навигации назад живёт в ComponentContext.
}

impl StateScreen {
    pub fn new() -> Self {
        Self { counter: 0 }
    }
}

impl LifecycleObserver for StateScreen {}

impl ComponentNode for StateScreen {
    fn render(
        &self,
        ui: &mut UiWrapper,
        dispatch: &::egui_android_framework::runtime::DynDispatcher,
        ctx: &ComponentContext,
    ) {
        let typed = dispatch.wrap::<StateScreenMsg>();
        Component::render(self, ui, &typed, ctx);
    }

    fn handle_dyn(&mut self, msg: Box<dyn std::any::Any + Send>, ctx: &mut ComponentContext) {
        if let Ok(typed) = msg.downcast::<StateScreenMsg>() {
            Component::handle(self, *typed, ctx);
        } else {
            log::error!("StateScreen::handle_dyn: ожидался StateScreenMsg, получен неизвестный");
        }
    }

    fn handle_back(&mut self, _ctx: &mut ComponentContext) -> BackAction {
        // Единая точка кастомной логики — и для рисованной, и для платформенной кнопки.
        self.counter = 0;
        BackAction::Pop
    }

    fn save_state(&self) -> Option<Box<dyn std::any::Any + Send>> {
        ::egui_android_framework::core::PersistentState::save_to_boxed(self)
    }

    fn restore_state(&mut self, state: Box<dyn std::any::Any + Send>) {
        ::egui_android_framework::core::PersistentState::restore_from_boxed(self, state);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl Component for StateScreen {
    type State = ();
    type Message = StateScreenMsg;

    fn render(
        &self,
        ui: &mut UiWrapper,
        dispatch: &Dispatcher<Self::Message>,
        _ctx: &ComponentContext,
    ) {
        let c = &Theme::current_from_ui(ui).colors;

        Column::new()
            .scrollable()
            .show(ui, dispatch, |ui, dispatch| {
                Text::new("Сохраняемое состояние (PersistentState)")
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);
                Spacer::new(8.0).render(ui, dispatch);

                Text::new("Сохраняемый счётчик (бизнес-данные):").render(ui, dispatch);

                Text::new(format!("Значение: {}", self.counter))
                    .text_color(c.on_secondary)
                    .modifier(Modifier::new().padding(12.0).background(c.secondary))
                    .render(ui, dispatch);

                Text::new("⚡ Переживёт поворот экрана!")
                    .modifier(Modifier::new().padding(4.0))
                    .render(ui, dispatch);

                Button::new("+1")
                    .on_click(StateScreenMsg::Increment)
                    .theme_colors(c.primary)
                    .text_color(c.on_primary)
                    .modifier(Modifier::new().fill_max_width().padding(8.0))
                    .render(ui, dispatch);

                Button::new("-1")
                    .on_click(StateScreenMsg::Decrement)
                    .theme_colors(c.primary)
                    .text_color(c.on_primary)
                    .modifier(Modifier::new().fill_max_width().padding(8.0))
                    .render(ui, dispatch);

                Button::new("Сброс")
                    .on_click(StateScreenMsg::Reset)
                    .theme_colors(c.secondary)
                    .text_color(c.on_secondary)
                    .modifier(Modifier::new().fill_max_width().padding(8.0))
                    .render(ui, dispatch);

                Spacer::new(16.0).render(ui, dispatch);

                // --- UI-состояние (remember) — НЕ сохраняется ---
                Text::new("UI-состояние (remember):").render(ui, dispatch);

                let expanded = remember(ui, "ss_ui_expanded", || false);

                Button::new(if *expanded.get() {
                    "Свернуть ▲"
                } else {
                    "Развернуть ▼"
                })
                .on_click_with({
                    let expanded = expanded.clone();
                    move |_ui, _dispatch| {
                        expanded.modify(|v| *v = !*v);
                    }
                })
                .theme_colors(c.secondary)
                .text_color(c.on_secondary)
                .modifier(Modifier::new().padding(8.0))
                .render(ui, dispatch);

                if *expanded.get() {
                    Text::new("Раскрытый контент (НЕ сохраняется)")
                        .text_color(c.on_secondary)
                        .modifier(Modifier::new().padding(12.0).background(c.secondary))
                        .render(ui, dispatch);
                }

                Text::new("Это состояние сбросится при повороте экрана")
                    .modifier(Modifier::new().padding(4.0))
                    .render(ui, dispatch);

                Spacer::new(16.0).render(ui, dispatch);
                Button::new("← Назад (сброс + назад)")
                    .on_click(StateScreenMsg::Back)
                    .theme_colors(c.primary)
                    .text_color(c.on_primary)
                    .modifier(Modifier::new().fill_max_width().padding(8.0))
                    .render(ui, dispatch);
            });
    }

    fn handle(&mut self, msg: Self::Message, ctx: &mut ComponentContext) {
        match msg {
            StateScreenMsg::Increment => self.counter += 1,
            StateScreenMsg::Decrement => self.counter -= 1,
            StateScreenMsg::Reset => self.counter = 0,
            StateScreenMsg::Back => {
                // Рисованная кнопка — делегируем в единую точку handle_back.
                self.handle_back(ctx);
            }
        }
    }

    fn state(&self) -> &Self::State {
        &()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn back_resets_counter_and_pops() {
        let mut screen = StateScreen::new();
        screen.counter = 42;
        let mut ctx = ComponentContext::new();
        assert_eq!(screen.handle_back(&mut ctx), BackAction::Pop);
        assert_eq!(screen.counter, 0);
    }

    #[test]
    fn handle_msg_back_delegates_to_handle_back() {
        let mut screen = StateScreen::new();
        screen.counter = 99;
        let mut ctx = ComponentContext::new();
        screen.handle(StateScreenMsg::Back, &mut ctx);
        // handle_back уже сбросил counter (через делегирование)
        assert_eq!(screen.counter, 0);
    }
}
