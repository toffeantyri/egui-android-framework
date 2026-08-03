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
use egui_android_framework::PersistentState;

/// Сообщения экрана состояния.
#[derive(Clone, Debug)]
pub enum StateScreenMsg {
    Increment,
    Decrement,
    Reset,
    Back,
}

#[derive(PersistentState)]
#[persistent_fields(counter)]
pub struct StateScreen {
    counter: i32,
    // Флаг Back теперь не нужен — единая точка логики в handle_back() -> BackAction.
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

    fn handle_dyn(
        &mut self,
        msg: Box<dyn std::any::Any + Send>,
        ctx: &mut ComponentContext,
    ) -> Option<BackAction> {
        if let Ok(typed) = msg.downcast::<StateScreenMsg>() {
            if matches!(&*typed, StateScreenMsg::Back) {
                // Не вызываем handle_back здесь — app.rs вызовет on_back(),
                // который дойдёт до handle_back через ChildStack::on_back().
                // Возвращаем Propagate, чтобы app.rs знал, что нужен pop.
                return Some(BackAction::Propagate);
            }
            // Обычное (не-навигационное) сообщение — обработано, навигация не требуется.
            Component::handle(self, *typed, ctx);
            return None;
        }
        log::error!("StateScreen::handle_dyn: ожидался StateScreenMsg, получен неизвестный");
        Some(BackAction::Propagate)
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

    fn handle(&mut self, msg: Self::Message, _ctx: &mut ComponentContext) {
        match msg {
            StateScreenMsg::Increment => self.counter += 1,
            StateScreenMsg::Decrement => self.counter -= 1,
            StateScreenMsg::Reset => self.counter = 0,
            // Back не обрабатывается в handle().
            // Рисованная кнопка Back → handle_dyn возвращает Propagate →
            // app.rs вызывает on_back() → handle_back().
            // Платформенная кнопка → on_back_pressed() → on_back() → handle_back().
            StateScreenMsg::Back => {}
        }
    }

    fn state(&self) -> &Self::State {
        &()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// handle_back — единая точка кастомной логики.
    /// Проверяет базовый контракт: сброс + Pop.
    #[test]
    fn back_resets_counter_and_pops() {
        let mut screen = StateScreen::new();
        screen.counter = 42;
        let mut ctx = ComponentContext::new();
        assert_eq!(screen.handle_back(&mut ctx), BackAction::Pop);
        assert_eq!(screen.counter, 0);
    }

    /// handle_dyn для Back НЕ вызывает handle_back.
    ///
    /// Контракт: handle_dyn должен вернуть Propagate и не менять состояние.
    /// app.rs получит Propagate → вызовет on_back() → handle_back()
    /// будет вызван ровно один раз через ChildStack::on_back().
    #[test]
    fn handle_dyn_back_returns_propagate_and_does_not_reset() {
        let mut screen = StateScreen::new();
        screen.counter = 99;
        let mut ctx = ComponentContext::new();
        let action = screen.handle_dyn(Box::new(StateScreenMsg::Back), &mut ctx);
        assert_eq!(
            action,
            Some(BackAction::Propagate),
            "handle_dyn для Back должен вернуть Some(Propagate)"
        );
        assert_eq!(
            screen.counter, 99,
            "handle_dyn НЕ должен вызывать handle_back — counter не сбрасывается"
        );
    }

    /// Обычное сообщение не должно интерпретироваться как навигационное.
    #[test]
    fn increment_does_not_trigger_back() {
        let mut screen = StateScreen::new();
        screen.counter = 10;
        let mut ctx = ComponentContext::new();
        let action = screen.handle_dyn(Box::new(StateScreenMsg::Increment), &mut ctx);
        assert_eq!(
            action, None,
            "обычное сообщение должно вернуть None (не навигация)"
        );
        assert_eq!(screen.counter, 11);
    }

    /// Полная цепочка рисованной кнопки Back — проверка на отсутствие двойного вызова.
    ///
    /// Имитирует реальный поток app.rs:frame():
    /// 1. handle_dyn(Back) → Propagate (состояние НЕ меняется)
    /// 2. app.rs: Propagate → on_back() → ChildStack::on_back()
    /// 3. ChildStack::on_back() → handle_back() — единственный вызов
    /// 4. handle_back: counter = 0; Pop
    ///
    /// Если бы handle_dyn сам вызывал handle_back (старый баг),
    /// то counter обнулился бы уже на шаге 1, и шаг 3 задвоил бы операцию.
    #[test]
    fn full_chain_handle_back_called_once() {
        let mut screen = StateScreen::new();
        screen.counter = 42;
        let mut ctx = ComponentContext::new();

        // Шаг 1: симуляция app.rs → active.handle_dyn(msg, ctx)
        let action = screen.handle_dyn(Box::new(StateScreenMsg::Back), &mut ctx);
        assert_eq!(action, Some(BackAction::Propagate));
        assert_eq!(screen.counter, 42, "шаг 1: handle_dyn НЕ меняет состояние");

        // Шаг 2: симуляция ChildStack::on_back(ctx) → handle_back
        let action2 = screen.handle_back(&mut ctx);
        assert_eq!(action2, BackAction::Pop);
        assert_eq!(
            screen.counter, 0,
            "шаг 2: handle_back вызван ровно один раз, counter сброшен"
        );

        // Если бы handle_back был вызван дважды (баг), counter уже был бы 0
        // на шаге 1, и здесь мы бы не заметили разницы (идемпотентность).
        // Ключевое доказательство — ассерт на шаге 1: counter == 42.
    }
}
