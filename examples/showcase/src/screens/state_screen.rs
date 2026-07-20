//! StateScreen — демонстрация сохраняемого состояния через PersistentState.
//!
//! Показывает разницу между:
//! - **PersistentState** (бизнес-данные) — сохраняется при повороте экрана
//! - **remember** (UI-состояние) — сбрасывается при повороте
//!
//! Аналог Decompose: `stateKeeper` для данных, `remember` для UI.

use egui_android_framework::core::{Component as UiComponent, LifecycleObserver, UiWrapper};
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
}

#[derive(Component)]
#[persistent_fields(counter)]
pub struct StateScreen {
    counter: i32,
}

impl StateScreen {
    pub fn new() -> Self {
        Self { counter: 0 }
    }
}

impl LifecycleObserver for StateScreen {}

impl UiComponent for StateScreen {
    type State = ();
    type Message = StateScreenMsg;

    fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<Self::Message>) {
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
                Button::new("← Назад")
                    .on_click(StateScreenMsg::Reset)
                    .theme_colors(c.primary)
                    .text_color(c.on_primary)
                    .modifier(Modifier::new().fill_max_width().padding(8.0))
                    .render(ui, dispatch);
            });
    }

    fn handle(&mut self, msg: Self::Message) {
        match msg {
            StateScreenMsg::Increment => self.counter += 1,
            StateScreenMsg::Decrement => self.counter -= 1,
            StateScreenMsg::Reset => self.counter = 0,
        }
    }

    fn state(&self) -> &Self::State {
        &()
    }
}
