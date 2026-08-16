//! TextSelectionScreen — демонстрация Android-подобного выделения текста.
//!
//! Показывает виджет `Text` в режиме `selectable(true)`:
//! - **Долгое нажатие** (~400мс, палец не смещается) выделяет слово под пальцем.
//! - **Ручки-капельки** на границах выделения расширяют/сужают диапазон.
//! - **Тулбар** над выделением позволяет скопировать текст (`Copy`) или
//!   выделить всё (`Всё`).
//! - Тап вне выделенной области снимает выделение.
//!
//! Ограничение P0: выделение НЕ работает внутри скролла (`LazyColumn`, `Column.scrollable`).
//! Поэтому этот экран НЕ оборачивается в scrollable — чтобы long-press перехватывался
//! текстом, а не уходил в скролл.

use egui_android_framework::core::{Component, ComponentContext, LifecycleObserver, UiWrapper};
use egui_android_framework::runtime::Dispatcher;
use egui_android_framework::ui::{
    containers::Column,
    modifier::{Modifier, ModifierDsl},
    theme::Theme,
    widgets::{Button, Spacer, Text, Widget},
};
use egui_android_framework::ComponentNode;

use crate::navigation_host::RootMsg;

/// Экран демонстрации выделения текста.
#[derive(ComponentNode)]
#[component_message(RootMsg)]
pub struct TextSelectionScreen;

impl TextSelectionScreen {
    pub fn new() -> Self {
        Self
    }
}

impl LifecycleObserver for TextSelectionScreen {}

impl Component for TextSelectionScreen {
    type State = ();
    type Message = RootMsg;

    fn render(
        &self,
        ui: &mut UiWrapper,
        dispatch: &Dispatcher<Self::Message>,
        _ctx: &ComponentContext,
    ) {
        let c = &Theme::current_from_ui(ui).colors;

        // БЕЗ `.scrollable()` — выделение работает только вне скролла (P0).
        Column::new()
            .spacing(8.0)
            .show(ui, dispatch, |ui, dispatch| {
                Text::new("Выделение текста")
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);
                Spacer::new(8.0).render(ui, dispatch);

                Text::new("Инструкция:")
                    .text_color(c.on_secondary)
                    .modifier(Modifier::new().padding(8.0).background(c.secondary))
                    .render(ui, dispatch);
                Text::new("1) Удерживайте палец ~0.4с на слове — оно выделится.")
                    .render(ui, dispatch);
                Text::new("2) Тяните капельки на границах, чтобы расширить выделение.")
                    .render(ui, dispatch);
                Text::new("3) Нажмите «Копировать», чтобы скопировать текст.").render(ui, dispatch);
                Text::new("4) Тап вне выделения снимает его.").render(ui, dispatch);
                Spacer::new(8.0).render(ui, dispatch);

                // ─── Пример 1: выделяемый текст ─────────────────────────────
                Text::new("Выделяемая строка (long-press → слово):")
                    .text_color(c.primary)
                    .render(ui, dispatch);
                Text::new(
                    "Сегодня я впервые использовал эту библиотеку на своём Android-устройстве.",
                )
                .modifier(
                    Modifier::new()
                        .selectable(true)
                        .padding(8.0)
                        .background(c.secondary),
                )
                .render(ui, dispatch);
                Text::new("Выделите любое слово и скопируйте его.")
                    .text_color(c.on_secondary)
                    .render(ui, dispatch);
                Spacer::new(8.0).render(ui, dispatch);

                // ─── Пример 2: центр ────────────────────────────────────────
                Text::new("Центрированный текст (align=Center):")
                    .text_color(c.primary)
                    .render(ui, dispatch);
                Text::new("Здесь тоже можно выделить слово долгим нажатием.")
                    .align(egui::Align::Center)
                    .modifier(
                        Modifier::new()
                            .selectable(true)
                            .fill_max_width()
                            .padding(8.0)
                            .background(c.secondary),
                    )
                    .render(ui, dispatch);
                Spacer::new(8.0).render(ui, dispatch);

                // ─── Пример 3: невыделяемый (сравнение) ─────────────────────
                Text::new("Невыделяемый текст (selectable=false, по умолчанию):")
                    .text_color(c.primary)
                    .render(ui, dispatch);
                Text::new("Долгое нажатие здесь не создаёт выделения — просто текст.")
                    .modifier(Modifier::new().padding(8.0).background(c.secondary))
                    .render(ui, dispatch);
                Spacer::new(8.0).render(ui, dispatch);

                Text::new("Примечание: выделение не работает внутри скролл-контейнеров (P0).")
                    .text_color(c.on_secondary)
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);
                Spacer::new(12.0).render(ui, dispatch);

                Button::new("← Назад")
                    .on_click(RootMsg::Back)
                    .theme_colors(c.secondary)
                    .text_color(c.on_secondary)
                    .modifier(Modifier::new().fill_max_width().padding(8.0))
                    .render(ui, dispatch);
            });
    }

    fn handle(&mut self, _msg: Self::Message, _ctx: &mut ComponentContext) {}

    fn state(&self) -> &Self::State {
        &()
    }
}
