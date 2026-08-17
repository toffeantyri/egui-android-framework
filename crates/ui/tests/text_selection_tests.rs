//! Интеграционные тесты Android-подобного выделения текста
//! (`Text::selectable(true)` и `TextEdit` + выделение).
//!
//! Чистая логика (распознавание long-press, выбор слова, позиции ручек, тулбар)
//! покрыта юнит-тестами в `crates/ui/src/text_selection/`. Здесь — проверка
//! интеграции в рендер `Text` / `TextEdit`.

use egui_android_core::{widget::Widget as WidgetTrait, UiWrapper};
use egui_android_runtime::Dispatcher;
use egui_android_ui::{Text, TextEdit};

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

/// Рендер `Text` с `selectable(true)` не должен паниковать (заглушка, этап 1).
#[test]
fn selectable_text_renders_without_panic() {
    with_ui(|ui| {
        let (dispatch, _rx) = Dispatcher::<()>::new();
        Text::new("hello selectable")
            .selectable(true)
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
            .selectable(true)
            .render(ui, &dispatch);
    });
}

/// `selectable(false)` — текст рендерится как обычно, без выделения.
#[test]
fn non_selectable_text_renders_unchanged() {
    with_ui(|ui| {
        let (dispatch, _rx) = Dispatcher::<()>::new();
        Text::new("обычный текст без выделения")
            .selectable(false)
            .render(ui, &dispatch);
    });
}

/// `selectable(true)` стабилен между кадрами (заглушка, состояние через `remember` на этапе 6).
#[test]
fn selectable_text_stable_across_frames() {
    let ctx = egui::Context::default();
    let run = |ctx: &egui::Context| {
        let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let (dispatch, _rx) = Dispatcher::<()>::new();
                Text::new("стабильный")
                    .selectable(true)
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
            .selectable(true)
            .render(ui, &dispatch);
        ui.add_space(8.0);
        Text::new("второй выделяемый")
            .selectable(true)
            .render(ui, &dispatch);
    });
}

/// Рендер редактируемого `TextEdit` с Android-выделением не паникует
/// (вне фокуса выделение не активно, но рендер-путь работает).
#[test]
fn textedit_selectable_renders_without_panic() {
    with_ui(|ui| {
        let (dispatch, _rx) = Dispatcher::<()>::new();
        TextEdit::<()>::new("редактируемый текст")
            .single_line()
            .render(ui, &dispatch);
    });
}

/// Read-only `TextEdit` тоже проходит selection-путь (Copy/SelectAll) без паники.
#[test]
fn textedit_readonly_renders_without_panic() {
    with_ui(|ui| {
        let (dispatch, _rx) = Dispatcher::<()>::new();
        TextEdit::<()>::new("read-only текст")
            .single_line()
            .read_only()
            .render(ui, &dispatch);
    });
}
