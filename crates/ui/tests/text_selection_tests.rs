//! Интеграционные тесты Android-подобного выделения текста
//! (`Text` + `Modifier::selectable(true)`).
//!
//! Чистая логика (распознавание long-press, выбор слова, позиции ручек, тулбар)
//! покрыта юнит-тестами в `crates/ui/src/text_selection/`. Здесь — проверка
//! интеграции модификатора в рендер `Text`.

use egui_android_core::{widget::Widget as WidgetTrait, UiWrapper};
use egui_android_runtime::Dispatcher;
use egui_android_ui::{Modifier, ModifierDsl, Text};

/// Запустить замыкание с реальным egui-ui (шрифты загружены).
fn with_ui(f: impl FnOnce(&mut UiWrapper)) {
    let f = std::cell::RefCell::new(Some(f));
    let ctx = egui::Context::default();
    let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let f = f.borrow_mut().take().unwrap();
            f(&mut UiWrapper::new_unconstrained(ui));
        });
    });
}

/// Рендер `Text` с `Modifier::selectable(true)` не должен паниковать (пустой прогон).
#[test]
fn selectable_text_renders_without_panic() {
    with_ui(|ui| {
        let (dispatch, _rx) = Dispatcher::<()>::new();
        Text::new("hello selectable")
            .modifier(Modifier::new().selectable(true))
            .render(ui, &dispatch);
    });
}

/// `selectable(true)` с `align` центрирует текст (не паникует).
#[test]
fn selectable_text_centered_renders() {
    with_ui(|ui| {
        let (dispatch, _rx) = Dispatcher::<()>::new();
        Text::new("Центрированный текст")
            .align(egui::Align::Center)
            .modifier(Modifier::new().selectable(true))
            .render(ui, &dispatch);
    });
}

/// `selectable(false)` — текст рендерится как обычно, без selection-слоя.
#[test]
fn non_selectable_text_renders_unchanged() {
    with_ui(|ui| {
        let (dispatch, _rx) = Dispatcher::<()>::new();
        Text::new("обычный текст без выделения")
            .modifier(Modifier::new().selectable(false))
            .render(ui, &dispatch);
    });
}

/// `selectable(true)` стабилен между кадрами (состояние через `remember`).
#[test]
fn selectable_text_stable_across_frames() {
    let ctx = egui::Context::default();
    let run = |ctx: &egui::Context| {
        let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let (dispatch, _rx) = Dispatcher::<()>::new();
                Text::new("стабильный")
                    .modifier(Modifier::new().selectable(true))
                    .render(&mut UiWrapper::new_unconstrained(ui), &dispatch);
            });
        });
    };
    run(&ctx);
    run(&ctx);
    run(&ctx);
}

/// `selectable(true)` для нескольких текстов на экране не паникует.
#[test]
fn multiple_selectable_texts_render() {
    with_ui(|ui| {
        let (dispatch, _rx) = Dispatcher::<()>::new();
        Text::new("первый выделяемый")
            .modifier(Modifier::new().selectable(true))
            .render(ui, &dispatch);
        ui.add_space(8.0);
        Text::new("второй выделяемый")
            .modifier(Modifier::new().selectable(true))
            .render(ui, &dispatch);
    });
}
