//! TextEditScreen — демонстрация виджета `TextEdit` в разных режимах.
//!
//! Показывает функционально разные варианты текстового ввода:
//! - **Single-line** (email) — однострочный, `KeyboardType::Email`, `ImeAction::Next`
//! - **Password** — маска пароля, `KeyboardType::Password`, `ImeAction::Done`
//! - **Multiline** — многострочный комментарий с `max_lines` и `char_limit`
//! - **Read-only** — только чтение (клавиатура не открывается)
//! - **Hint** — пустое поле с подсказкой-плейсхолдером
//!
//! Текст редактируется через `remember` (локальное UI-состояние, не сохраняется
//! при kill/restore) в комбинации с `on_changed`. `on_submit` демонстрирует
//! отправку готового значения через `dispatch` при Done / потере фокуса.

use egui_android_framework::core::{Component, ComponentContext, LifecycleObserver, UiWrapper};
use egui_android_framework::runtime::Dispatcher;
use egui_android_framework::ui::{
    containers::Column,
    modifier::{Modifier, ModifierDsl},
    remember,
    theme::Theme,
    widgets::{Button, ImeAction, KeyboardType, Spacer, Text, TextEdit, Widget},
};
use egui_android_framework::ComponentNode;

use crate::navigation_host::RootMsg;

/// Экран демонстрации TextEdit.
#[derive(ComponentNode)]
#[component_message(RootMsg)]
pub struct TextEditScreen;

impl TextEditScreen {
    pub fn new() -> Self {
        Self
    }
}

impl LifecycleObserver for TextEditScreen {}

impl Component for TextEditScreen {
    type State = ();
    type Message = RootMsg;

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
                Text::new("TextEdit")
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);
                Spacer::new(8.0).render(ui, dispatch);
                Text::new("Текстовый ввод в разных режимах:").render(ui, dispatch);

                // ─── 1. Single-line (Email) ───────────────────────────────
                Text::new("1. Однострочный (Email):").render(ui, dispatch);
                let email = remember(ui, "te_email", || String::new());
                // ВАЖНО: read-guard от `email.get()` держим ТОЛЬКО в локальной
                // переменной, а не в одном выражении с `.render()`. Если построить
                // `TextEdit::new(email.get().clone()).on_changed(move |v| email.set(v))...
                // .render(...)` в одном полном выражении, временный `RwLockReadGuard`
                // живёт до конца `render()`, а `on_changed -> email.set` берёт write на
                // тот же std::sync::RwLock -> самоблокировка (self-deadlock).
                let email_init = email.get().clone();
                let email = email.clone();
                TextEdit::new(email_init)
                    .selectable(true)
                    .hint("example@mail.com")
                    .single_line()
                    .keyboard_type(KeyboardType::Email)
                    .ime_action(ImeAction::Next)
                    .on_changed(move |v| email.set(v.to_owned()))
                    .modifier(Modifier::new().fill_max_width().padding_hv(12.0, 10.0))
                    .render(ui, dispatch);

                // ─── 2. Password ─────────────────────────────────────────
                Text::new("2. Пароль (маска):").render(ui, dispatch);
                let password = remember(ui, "te_password", || String::new());
                let password_init = password.get().clone();
                let password = password.clone();
                TextEdit::new(password_init)
                    .hint("••••••••")
                    .single_line()
                    .password()
                    .keyboard_type(KeyboardType::Password)
                    .ime_action(ImeAction::Done)
                    .on_changed(move |v| password.set(v.to_owned()))
                    .modifier(Modifier::new().fill_max_width().padding_hv(12.0, 10.0))
                    .render(ui, dispatch);

                // ─── 3. Multiline комментарий ───────────────────────────
                Text::new("3. Многострочный комментарий (max 4 строки):").render(ui, dispatch);
                let comment = remember(ui, "te_comment", || String::new());
                let comment_init = comment.get().clone();
                let comment = comment.clone();
                TextEdit::new(comment_init)
                    .hint("Введите комментарий...")
                    .multiline()
                    .max_lines(4)
                    .on_changed(move |v| comment.set(v.to_owned()))
                    .modifier(Modifier::new().fill_max_width().padding_hv(12.0, 10.0))
                    .render(ui, dispatch);

                // ─── 4. Read-only ────────────────────────────────────────
                Text::new("4. Только чтение (read-only):").render(ui, dispatch);
                TextEdit::new("Это поле нельзя редактировать")
                    .single_line()
                    .read_only()
                    .modifier(Modifier::new().fill_max_width().padding_hv(12.0, 10.0))
                    .render(ui, dispatch);

                // ─── 5. Hint (пустое поле с подсказкой) ─────────────────
                Text::new("5. Подсказка (placeholder):").render(ui, dispatch);
                let query = remember(ui, "te_query", || String::new());
                let last_submit = remember(ui, "te_last_submit", || String::new());
                // Read-guard от `query.get()` разрываем до построения/рендера,
                // чтобы `on_changed -> query.set` не самоблокировался на RwLock.
                let query_init = query.get().clone();
                let query = query.clone();
                TextEdit::new(query_init)
                    .hint("Поиск...")
                    .single_line()
                    .keyboard_type(KeyboardType::Text)
                    .ime_action(ImeAction::Search)
                    .on_changed(move |v| query.set(v.to_owned()))
                    .on_submit({
                        // `last_submit` остаётся в области видимости для отрисовки
                        // результата; в замыкание уходит clone.
                        let last = last_submit.clone();
                        move |v| last.set(v.to_owned())
                    })
                    .modifier(Modifier::new().fill_max_width().padding_hv(12.0, 10.0))
                    .render(ui, dispatch);

                Spacer::new(16.0).render(ui, dispatch);

                // ─── Значение по Submit (паттерн on_submit) ─────────────
                Text::new("Результат on_submit (Done / потеря фокуса):").render(ui, dispatch);
                Text::new(if last_submit.get().is_empty() {
                    "(нет — введите текст в поле поиска и нажмите Done)".to_owned()
                } else {
                    format!("\"{}\" ✓", last_submit.get())
                })
                .text_color(c.on_secondary)
                .modifier(Modifier::new().padding(12.0).background(c.secondary))
                .render(ui, dispatch);

                Spacer::new(16.0).render(ui, dispatch);
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
