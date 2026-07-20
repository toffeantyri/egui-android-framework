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

use crate::navigation_host::RootMsg;

/// Сохраняемый счётчик.
///
/// Поле `counter` помечено как `persistent` — его значение
/// сохранится при повороте экрана через bincode → Bundle.
#[derive(Component)]
#[persistent_fields(counter)]
pub struct StateScreen {
    /// Бизнес-данные: счётчик (сохраняется)
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
    type Message = RootMsg;

    fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<Self::Message>) {
        let c = &Theme::current_from_ui(ui).colors;

        Column::new()
            .scrollable()
            .show(ui, dispatch, |ui, dispatch| {
                Text::new("Сохраняемое состояние (PersistentState)")
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);
                Spacer::new(8.0).render(ui, dispatch);

                // --- Сохраняемый счётчик ---
                Text::new("Сохраняемый счётчик (бизнес-данные):").render(ui, dispatch);

                Text::new(format!("Значение: {}", self.counter))
                    .text_color(c.on_secondary)
                    .modifier(Modifier::new().padding(12.0).background(c.secondary))
                    .render(ui, dispatch);

                Text::new("⚡ Переживёт поворот экрана!")
                    .modifier(Modifier::new().padding(4.0))
                    .render(ui, dispatch);

                // Кнопки инкремента через MVI
                Button::new("+1 (бизнес-сообщение)")
                    .on_click(RootMsg::ToggleTheme) // временно
                    .theme_colors(c.primary)
                    .text_color(c.on_primary)
                    .modifier(Modifier::new().fill_max_width().padding(8.0))
                    .render(ui, dispatch);

                Text::new("Нажми +1 несколько раз, затем поверни экран — счётчик сохранится")
                    .modifier(Modifier::new().padding(4.0))
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
                    .on_click(RootMsg::Back)
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
