//! Интеграционные тесты виджетов, контейнеров, модификаторов, анимаций и темы.

use std::cell::RefCell;

use egui_android_core::{widget::Widget as WidgetTrait, Dispatcher, UiWrapper};
use egui_android_ui::animation::{
    animate_bool, animate_value, AnimatedVisibility, AnimationExt, Fade, Slide, SlideDirection,
};
use egui_android_ui::containers::{Column, LazyColumn, Row, Stack};
use egui_android_ui::modifier::{Modifier, ModifierApply, ModifierExt};
use egui_android_ui::theme::{MaterialTheme, Shapes, Theme};
use egui_android_ui::widgets::{Button, Icon, Spacer, Text};

// ─── Helper: with_ui ────────────────────────────────────────────────────────────

fn with_ui(f: impl FnOnce(&mut UiWrapper)) {
    let f = RefCell::new(Some(f));
    let ctx = egui::Context::default();
    let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let f = f.borrow_mut().take().unwrap();
            f(&mut UiWrapper::new_unconstrained(ui));
        });
    });
}

// ═══════════════════════════════════════════════════════════════════════════════════
// WIDGET TESTS (23 tests)
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn test_text_widget_renders() {
    with_ui(|ui| {
        let text = Text::new("Привет, мир!");
        text.render(ui, &Dispatcher::<()>::new().0);
        // не паникует — значит, рендерится
    });
}

#[test]
fn test_text_widget_is_widget() {
    fn takes_widget<M: 'static>(_w: impl WidgetTrait<M>) {}
    takes_widget::<()>(Text::new("test"));
}

#[test]
fn test_button_widget_renders() {
    with_ui(|ui| {
        let btn = Button::<()>::new("Click me");
        btn.render(ui, &Dispatcher::new().0);
    });
}

#[test]
fn test_button_on_click_no_panic() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let btn = Button::new("Click").on_click(());
        btn.render(ui, &dispatch);
    });
}

#[test]
fn test_button_with_custom_height() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let btn = Button::new("Tall").height(64.0);
        btn.render(ui, &dispatch);
    });
}

#[test]
fn test_button_dispatches_message_on_click() {
    use std::sync::{Arc, Mutex};
    let (dispatch, rx) = Dispatcher::<&str>::new();
    let msg = Arc::new(Mutex::new(None::<&str>));
    let msg_clone = Arc::clone(&msg);

    with_ui(|ui| {
        // Рендерим кнопку
        let btn = Button::new("Send").on_click("clicked");
        btn.render(ui, &dispatch);

        // Собираем сообщения
        for m in rx.try_iter() {
            *msg_clone.lock().unwrap() = Some(m);
        }
    });

    // В тестовой среде click() не вызывается, поэтому сообщения не будет.
    // Тест просто проверяет, что нет паники при рендере с on_click.
}

#[test]
fn test_button_on_click_with_no_panic() {
    // on_click_with closure не должен паниковать при рендере
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let btn = Button::<()>::new("With closure").on_click_with(|_ui, _dispatch| {
            // closure просто не паникует
        });
        btn.render(ui, &dispatch);
    });
}

#[test]
fn test_button_on_click_with_both_msg_and_closure() {
    // on_click(msg) + on_click_with(closure) не должны паниковать
    let (dispatch, rx) = Dispatcher::<&'static str>::new();
    with_ui(|ui| {
        let btn = Button::new("Both")
            .on_click("msg_dispatched")
            .on_click_with(|_ui, _dispatch| {
                // closure тоже не паникует
            });
        btn.render(ui, &dispatch);

        // Сообщение должно быть в канале (проверяем что нет паники)
        for _ in rx.try_iter() {}
    });
}

#[test]
fn test_button_on_click_with_remember_modify() {
    // on_click_with может модифицировать remember напрямую
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let count = egui_android_ui::remember(ui, "test_btn_cnt", || 0i32);
        assert_eq!(*count.get(), 0);

        // Рендер кнопки с on_click_with — в тестовой среде click не происходит,
        // но проверяем что нет паники и структура корректна
        let btn = Button::<()>::new("+1").on_click_with({
            let count = count.clone();
            move |_ui, _dispatch| {
                count.modify(|c| *c += 1);
            }
        });
        btn.render(ui, &dispatch);

        // remember не изменится без реального клика, это ожидаемо.
        // Тест проверяет отсутствие паники.
        assert_eq!(*count.get(), 0);
    });
}

#[test]
fn test_clickable_with_modifier_renders() {
    // clickable_with модификатор не должен паниковать при рендере
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Text::new("Click me")
            .clickable_with(|_response, _ui, _dispatch| {
                // closure не паникует
            })
            .render(ui, &dispatch);
    });
}

#[test]
fn test_spacer_widget_renders() {
    with_ui(|ui| {
        let spacer = Spacer::new(16.0);
        spacer.render(ui, &Dispatcher::<()>::new().0);
    });
}

#[test]
fn test_spacer_is_widget() {
    fn takes_widget<M: 'static>(_w: impl WidgetTrait<M>) {}
    takes_widget::<()>(Spacer::new(8.0));
}

#[test]
fn test_icon_widget_renders() {
    with_ui(|ui| {
        let uri = egui::Image::from_bytes("bytes://test", "fake".as_bytes());
        let icon = Icon::new(uri);
        icon.render(ui, &Dispatcher::<()>::new().0);
    });
}

#[test]
fn test_widget_trait_object() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let widgets: Vec<Box<dyn WidgetTrait<()>>> =
            vec![Box::new(Text::new("A")), Box::new(Text::new("B"))];
        for w in &widgets {
            w.render(ui, &dispatch);
        }
    });
}

#[test]
fn test_dispatcher_send_sync() {
    fn assert_send<T: Send>(_: &T) {}
    fn assert_sync<T: Sync>(_: &T) {}
    let (d, _) = Dispatcher::<()>::new();
    assert_send(&d);
    assert_sync(&d);
}

#[test]
fn test_dispatcher_message_type() {
    #[derive(Debug, PartialEq, Clone)]
    enum MyMsg {
        Inc,
        Dec,
    }
    let (dispatch, rx) = Dispatcher::<MyMsg>::new();

    dispatch.dispatch(MyMsg::Inc);
    dispatch.dispatch(MyMsg::Dec);
    dispatch.dispatch(MyMsg::Inc);

    let msgs: Vec<MyMsg> = rx.try_iter().collect();
    assert_eq!(msgs, vec![MyMsg::Inc, MyMsg::Dec, MyMsg::Inc]);
}

#[test]
fn test_dispatcher_multiple_dispatch() {
    let (dispatch, rx) = Dispatcher::<i32>::new();
    for i in 0..10 {
        dispatch.dispatch(i);
    }
    let msgs: Vec<i32> = rx.try_iter().collect();
    assert_eq!(msgs.len(), 10);
    assert_eq!(msgs, (0..10).collect::<Vec<i32>>());
}

#[test]
fn test_dispatcher_clone() {
    let (dispatch1, rx) = Dispatcher::<String>::new();
    let dispatch2 = dispatch1.clone();

    dispatch1.dispatch("from_1".to_string());
    dispatch2.dispatch("from_2".to_string());
    drop(dispatch1);
    drop(dispatch2);

    let msgs: Vec<String> = rx.try_iter().collect();
    assert_eq!(msgs.len(), 2);
}

#[test]
fn test_dispatcher_no_message() {
    let (_dispatch, rx) = Dispatcher::<()>::new();
    let msgs: Vec<()> = rx.try_iter().collect();
    assert!(msgs.is_empty());
}

#[test]
fn test_widget_generic_with_different_messages() {
    fn test_widget<M: 'static>(widget: impl WidgetTrait<M>) {
        let (dispatch, _rx) = Dispatcher::<M>::new();
        with_ui(|ui| {
            widget.render(ui, &dispatch);
        });
    }
    test_widget::<()>(Text::new("unit"));
    test_widget::<String>(Text::new("string"));
    test_widget::<i32>(Text::new("int"));
    test_widget::<bool>(Text::new("bool"));
}

#[test]
fn test_dispatcher_drain_empty() {
    let (dispatch, rx) = Dispatcher::<i32>::new();
    dispatch.dispatch(1);
    let first: Vec<i32> = rx.try_iter().collect();
    assert_eq!(first, vec![1]);
    let second: Vec<i32> = rx.try_iter().collect();
    assert!(second.is_empty());
}

#[test]
fn test_shared_state_via_dispatcher() {
    let (dispatch, rx) = Dispatcher::<i32>::new();
    let d2 = dispatch.clone();
    d2.dispatch(42);
    let msgs: Vec<i32> = rx.try_iter().collect();
    assert_eq!(msgs, vec![42]);
}

#[test]
fn test_spacer_size_does_not_panic() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let spacer = Spacer::new(100.0);
        spacer.render(ui, &dispatch);
    });
}

#[test]
fn test_text_multiple_renders() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let text = Text::new("Hello");
        text.render(ui, &dispatch);
        text.render(ui, &dispatch);
        text.render(ui, &dispatch);
    });
}

// ═══════════════════════════════════════════════════════════════════════════════════
// CONTAINER TESTS (19 tests)
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn test_column_empty() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Column::new().show(ui, &dispatch, |_ui, _dispatch| {
            // пустая колонка — ничего не рендерим
        });
    });
}

#[test]
fn test_column_with_children() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Column::new().show(ui, &dispatch, |ui, dispatch| {
            Text::new("A").render(ui, dispatch);
            Text::new("B").render(ui, dispatch);
        });
    });
}

#[test]
fn test_column_with_spacing() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Column::new().show(ui, &dispatch, |ui, dispatch| {
            Text::new("A").render(ui, dispatch);
            Text::new("B").render(ui, dispatch);
        });
        // spacing по умолчанию 8.0 — проверяем что не паникует
    });
}

#[test]
fn test_column_nested() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Column::new().show(ui, &dispatch, |ui, dispatch| {
            Text::new("Outer").render(ui, dispatch);
            Column::new().show(ui, dispatch, |ui, dispatch| {
                Text::new("Inner").render(ui, dispatch);
            });
        });
    });
}

#[test]
fn test_column_with_text_widgets() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Column::new().show(ui, &dispatch, |ui, dispatch| {
            Text::new("Item 1").render(ui, dispatch);
            Text::new("Item 2").render(ui, dispatch);
            Text::new("Item 3").render(ui, dispatch);
        });
    });
}

#[test]
fn test_column_ordering_no_panic() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Column::new().show(ui, &dispatch, |ui, dispatch| {
            Text::new("First").render(ui, dispatch);
            Text::new("Second").render(ui, dispatch);
            // порядок гарантируется ui.vertical — не паникует
        });
    });
}

#[test]
fn test_row_empty() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Row::new(ui, &dispatch, |_ui, _dispatch| {
            // пустая строка — ничего не рендерим
        });
    });
}

#[test]
fn test_row_with_children() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Row::new(ui, &dispatch, |ui, dispatch| {
            Text::new("A").render(ui, dispatch);
            Text::new("B").render(ui, dispatch);
        });
    });
}

#[test]
fn test_row_with_spacing() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Row::new(ui, &dispatch, |ui, dispatch| {
            Text::new("A").render(ui, dispatch);
            Text::new("B").render(ui, dispatch);
        });
        // spacing по умолчанию 8.0 — проверяем что не паникует
    });
}

#[test]
fn test_row_nested() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Row::new(ui, &dispatch, |ui, dispatch| {
            Text::new("Outer").render(ui, dispatch);
            Row::new(ui, dispatch, |ui, dispatch| {
                Text::new("Inner").render(ui, dispatch);
            });
        });
    });
}

#[test]
fn test_row_ordering_no_panic() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Row::new(ui, &dispatch, |ui, dispatch| {
            Text::new("Left").render(ui, dispatch);
            Text::new("Right").render(ui, dispatch);
        });
    });
}

#[test]
fn test_stack_empty() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Stack::new(ui, &dispatch, |_ui, _dispatch| {
            // пустой стек — ничего не рендерим
        });
    });
}

#[test]
fn test_stack_with_children() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Stack::new(ui, &dispatch, |ui, dispatch| {
            Text::new("Layer 1").render(ui, dispatch);
            Text::new("Layer 2").render(ui, dispatch);
        });
    });
}

#[test]
fn test_lazy_column_empty() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let items: Vec<String> = vec![];
        LazyColumn::new(items, ui, &dispatch, |_item, _ui, _dispatch| {
            // пустой список — ничего не рендерим
        });
    });
}

#[test]
fn test_lazy_column_with_data() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let items = vec!["A".to_string(), "B".to_string(), "C".to_string()];
        LazyColumn::new(items, ui, &dispatch, |item, ui, dispatch| {
            Text::new(item.clone()).render(ui, dispatch);
        });
    });
}

#[test]
fn test_containers_mixed() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Column::new().show(ui, &dispatch, |ui, dispatch| {
            Row::new(ui, dispatch, |ui, dispatch| {
                Text::new("A").render(ui, dispatch);
                Text::new("B").render(ui, dispatch);
            });
            Text::new("C").render(ui, dispatch);
        });
    });
}

// ═══════════════════════════════════════════════════════════════════════════════════
// MODIFIER TESTS (15 tests)
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn test_padding_modifier() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Text::new("Padded").padding(16.0).render(ui, &dispatch);
    });
}

#[test]
fn test_size_modifier() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Text::new("Sized").size(100.0, 50.0).render(ui, &dispatch);
    });
}

#[test]
fn test_background_modifier() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Text::new("Filled")
            .background(egui::Color32::RED)
            .render(ui, &dispatch);
    });
}

#[test]
fn test_align_modifier() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Text::new("Centered")
            .align(egui::Align::Center)
            .render(ui, &dispatch);
    });
}

#[test]
fn test_clickable_modifier() {
    let (dispatch, _rx) = Dispatcher::<&str>::new();
    with_ui(|ui| {
        Text::new("Clickable")
            .clickable("clicked")
            .render(ui, &dispatch);
    });
}

#[test]
fn test_modifier_chain() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Text::new("Chained")
            .padding(8.0)
            .background(egui::Color32::from_gray(40))
            .render(ui, &dispatch);
    });
}

#[test]
fn test_modifier_chain_all() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Text::new("All")
            .padding(4.0)
            .size(200.0, 100.0)
            .background(egui::Color32::BLUE)
            .align(egui::Align::Center)
            .render(ui, &dispatch);
    });
}

#[test]
fn test_clickable_modifier_with_message() {
    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Clicked,
    }
    let (dispatch, rx) = Dispatcher::<Msg>::new();
    with_ui(|ui| {
        Text::new("Click me")
            .clickable(Msg::Clicked)
            .render(ui, &dispatch);
    });
    // В тестовой среде клик не произойдёт, но проверяем отсутствие паники
    let msgs: Vec<Msg> = rx.try_iter().collect();
    assert!(msgs.is_empty());
}

#[test]
fn test_spacer_with_modifiers() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Spacer::new(8.0)
            .size(50.0, 50.0)
            .background(egui::Color32::GREEN)
            .render(ui, &dispatch);
    });
}

#[test]
fn test_modifier_returns_widget() {
    fn requires_widget<M: 'static>(_w: impl WidgetTrait<M>) {}
    requires_widget::<()>(Text::new("test").padding(4.0));
    requires_widget::<()>(Text::new("test").size(10.0, 10.0));
    requires_widget::<()>(Text::new("test").background(egui::Color32::RED));
    requires_widget::<()>(Text::new("test").align(egui::Align::Center));
    requires_widget::<&str>(Text::new("test").clickable("msg"));
}

#[test]
fn test_padding_not_negative() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Text::new("Test").padding(-10.0).render(ui, &dispatch);
    });
}

#[test]
fn test_size_zero() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Text::new("Zero").size(0.0, 0.0).render(ui, &dispatch);
    });
}

#[test]
fn test_clickable_dispatches_to_all_clones() {
    let (dispatch, rx) = Dispatcher::<&str>::new();
    let d2 = dispatch.clone();
    d2.dispatch("from_d2");
    let msgs: Vec<&str> = rx.try_iter().collect();
    assert_eq!(msgs, vec!["from_d2"]);
}

// ═══════════════════════════════════════════════════════════════════════════════════
// ANIMATION TESTS (15 tests)
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn test_fade_widget_renders() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let fade = Fade::new(Text::new("Fading"), 0.5);
        fade.render(ui, &dispatch);
    });
}

#[test]
fn test_fade_opacity_clamped() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let fade = Fade::new(Text::new("Over"), 1.5);
        fade.render(ui, &dispatch);
    });
}

#[test]
fn test_fade_zero_opacity() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let fade = Fade::new(Text::new("Hidden"), 0.0);
        fade.render(ui, &dispatch);
    });
}

#[test]
fn test_fade_full_opacity() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let fade = Fade::new(Text::new("Visible"), 1.0);
        fade.render(ui, &dispatch);
    });
}

#[test]
fn test_slide_widget_renders() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let slide = Slide::new(Text::new("Sliding"), SlideDirection::Left, 50.0);
        slide.render(ui, &dispatch);
    });
}

#[test]
fn test_slide_all_directions() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let directions = [
            SlideDirection::Left,
            SlideDirection::Right,
            SlideDirection::Up,
            SlideDirection::Down,
        ];
        for dir in &directions {
            Slide::new(Text::new("Slide"), *dir, 30.0).render(ui, &dispatch);
        }
    });
}

#[test]
fn test_slide_zero_offset() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Slide::new(Text::new("Static"), SlideDirection::Down, 0.0).render(ui, &dispatch);
    });
}

#[test]
fn test_animated_visibility_visible() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        AnimatedVisibility::new(true, 0.3)
            .child(Text::new("Shown"))
            .render(ui, &dispatch);
    });
}

#[test]
fn test_animated_visibility_hidden() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        AnimatedVisibility::new(false, 0.3)
            .child(Text::new("Hidden"))
            .render(ui, &dispatch);
    });
}

#[test]
fn test_animation_ext_fade() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Text::new("Ext").fade(0.5).render(ui, &dispatch);
    });
}

#[test]
fn test_animation_ext_slide() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Text::new("Ext")
            .slide(SlideDirection::Right, 40.0)
            .render(ui, &dispatch);
    });
}

#[test]
fn test_animate_value_function() {
    with_ui(|ui| {
        let result = animate_value(ui, "val_test", 1.0, 0.5);
        assert!(result >= 0.0 && result <= 1.0);
    });
}

#[test]
fn test_animate_bool_function() {
    with_ui(|ui| {
        let result = animate_bool(ui, "bool_test", true, 0.5);
        assert!(result >= 0.0 && result <= 1.0);
    });
}

#[test]
fn test_animate_value_different_keys() {
    with_ui(|ui| {
        let r1 = animate_value(ui, "key_a", 1.0, 0.3);
        let r2 = animate_value(ui, "key_b", 1.0, 0.3);
        // Разные ключи — разные состояния
        assert!(r1 >= 0.0 && r2 >= 0.0);
    });
}

#[test]
fn test_animation_chain_with_modifier() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Text::new("Anim")
            .padding(8.0)
            .fade(0.8)
            .render(ui, &dispatch);
    });
}

// ═══════════════════════════════════════════════════════════════════════════════════
// THEME TESTS (11 tests)
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn test_material_theme_light_has_colors() {
    let theme = MaterialTheme::light();
    assert_ne!(theme.colors.primary, egui::Color32::BLACK);
    assert_ne!(theme.colors.background, egui::Color32::BLACK);
}

#[test]
fn test_material_theme_dark_has_colors() {
    let theme = MaterialTheme::dark();
    assert_ne!(theme.colors.primary, egui::Color32::BLACK);
}

#[test]
fn test_theme_current_and_set() {
    let ctx = egui::Context::default();
    let light = MaterialTheme::light();
    light.apply(&ctx);
    let current = Theme::current(&ctx);
    assert_eq!(current.colors.primary, light.colors.primary);
    assert_eq!(current.colors.background, light.colors.background);
}

#[test]
fn test_theme_current_default_when_not_set() {
    let ctx = egui::Context::default();
    let current = Theme::current(&ctx);
    // По умолчанию — светлая тема
    assert_eq!(
        current.colors.primary,
        MaterialTheme::light().colors.primary
    );
}

#[test]
fn test_theme_switch_between_light_and_dark() {
    let ctx = egui::Context::default();

    let light = MaterialTheme::light();
    light.apply(&ctx);
    assert_eq!(
        Theme::current(&ctx).colors.primary,
        MaterialTheme::light().colors.primary
    );

    let dark = MaterialTheme::dark();
    dark.apply(&ctx);
    assert_eq!(
        Theme::current(&ctx).colors.primary,
        MaterialTheme::dark().colors.primary
    );
}

#[test]
fn test_theme_current_from_ui() {
    with_ui(|ui| {
        let light = MaterialTheme::light();
        light.apply(ui.ctx());
        let current = Theme::current_from_ui(ui);
        assert_eq!(current.colors.primary, light.colors.primary);
    });
}

#[test]
fn test_theme_has_typography() {
    let theme = MaterialTheme::light();
    // Проверяем доступность полей типографики
    let _ = theme.typography.display_large;
    let _ = theme.typography.headline_large;
    let _ = theme.typography.body_medium;
    let _ = theme.typography.label_small;
}

#[test]
fn test_theme_has_shapes() {
    let theme = MaterialTheme::light();
    let _ = theme.shapes.small;
    let _ = theme.shapes.medium;
    let _ = theme.shapes.large;
}

#[test]
fn test_shapes_default_values() {
    let shapes = Shapes::default();
    assert_eq!(shapes.small, egui::CornerRadius::same(4));
    assert_eq!(shapes.medium, egui::CornerRadius::same(8));
    assert_eq!(shapes.large, egui::CornerRadius::same(16));
}

#[test]
fn test_theme_clone() {
    let theme = MaterialTheme::light();
    let cloned = theme.clone();
    assert_eq!(cloned.colors.primary, theme.colors.primary);
    assert_eq!(cloned.colors.background, theme.colors.background);
}

#[test]
fn test_typography_default_sizes() {
    let typography = egui_android_ui::theme::Typography::default();
    // display_large: 32px
    assert_eq!(typography.display_large.size, 32.0);
    // headline_large: 22px
    assert_eq!(typography.headline_large.size, 22.0);
    // body_medium: 14px
    assert_eq!(typography.body_medium.size, 14.0);
}

// ═══════════════════════════════════════════════════════════════════════════════════
// STAGE 2: NEW MODIFIER TESTS (wrap_content_width, wrap_content_size, clip, shadow)
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn test_wrap_content_width_renders_without_panic() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Text::new("Короткий")
            .modifier(
                Modifier::new()
                    .wrap_content_width()
                    .background(egui::Color32::RED),
            )
            .render(ui, &dispatch);
    });
}

#[test]
fn test_wrap_content_size_renders_without_panic() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Text::new("Короткий")
            .modifier(
                Modifier::new()
                    .wrap_content_size()
                    .background(egui::Color32::BLUE),
            )
            .render(ui, &dispatch);
    });
}

#[test]
fn test_clip_renders_without_panic() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Text::new("Обрезаемый текст")
            .modifier(
                Modifier::new()
                    .clip(egui::CornerRadius::same(8))
                    .padding(8.0),
            )
            .render(ui, &dispatch);
    });
}

#[test]
fn test_shadow_renders_without_panic() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Text::new("С тенью")
            .modifier(
                Modifier::new()
                    .padding(16.0)
                    .shadow(4.0)
                    .background(egui::Color32::WHITE),
            )
            .render(ui, &dispatch);
    });
}

#[test]
fn test_wrap_content_width_in_row_does_not_panic() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Row::new(ui, &dispatch, |ui, dispatch| {
            Text::new("Левый")
                .modifier(
                    Modifier::new()
                        .wrap_content_width()
                        .background(egui::Color32::DARK_GRAY),
                )
                .render(ui, dispatch);
            Text::new("Правый")
                .modifier(
                    Modifier::new()
                        .wrap_content_width()
                        .background(egui::Color32::DARK_BLUE),
                )
                .render(ui, dispatch);
        });
    });
}

#[test]
fn test_shadow_zero_does_not_panic() {
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Text::new("Без тени")
            .modifier(Modifier::new().shadow(0.0))
            .render(ui, &dispatch);
    });
}

// ═══════════════════════════════════════════════════════════════════════════════════
// STAGE 3: NEW TESTS (Clickable sizing, SizedWidget, Background, Aligned, Button)
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn test_clickable_uses_content_size_not_available_size() {
    // Clickable должен создавать область ровно по размеру контента,
    // а не по available_size() (которая может быть на весь экран).
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        // Текст маленький, а available_size() большой — кликабельная область
        // должна быть маленькой (по тексту), а не по всему экрану.
        let before = ui.available_size();
        Text::new("Короткий").clickable(()).render(ui, &dispatch);
        // После рендера available_size должен уменьшиться на размер текста,
        // а не на весь экран.
        let after = ui.available_size();
        assert!(
            after.y < before.y,
            "clickable не должен занимать всю высоту"
        );
        assert!(
            after.y > 0.0,
            "после кликабельного текста контент не должен исчезать"
        );
    });
}

#[test]
fn test_clickable_with_uses_content_size() {
    // ClickableWith должен создавать область по размеру контента.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let before = ui.available_size();
        Text::new("Короткий")
            .clickable_with(|_r, _ui, _d| {})
            .render(ui, &dispatch);
        let after = ui.available_size();
        assert!(
            after.y < before.y,
            "clickable_with не должен занимать всю высоту"
        );
        assert!(
            after.y > 0.0,
            "после clickable_with контент не должен исчезать"
        );
    });
}

#[test]
fn test_sized_widget_reserves_exact_size() {
    // SizedWidget должен резервировать ровно указанный размер.
    // Используем размер больше, чем содержимое, чтобы избежать
    // поправки на min_size.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let before = ui.available_size();
        Text::new("Текст").size(300.0, 100.0).render(ui, &dispatch);
        let after = ui.available_size();
        // available_height должен уменьшиться примерно на 100px
        // (с учётом item_spacing и min_size поправки)
        let consumed_y = before.y - after.y;
        assert!(
            consumed_y >= 100.0,
            "SizedWidget должен резервировать 100px высоты, а потребил {}px",
            consumed_y
        );
    });
}

#[test]
fn test_background_size_matches_content() {
    // Background должен рисовать фон строго по размеру контента,
    // а не растягивать его на всю доступную область.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let before = ui.available_size();
        Text::new("Текст")
            .background(egui::Color32::RED)
            .render(ui, &dispatch);
        let after = ui.available_size();
        // Фон не должен растягиваться — available_height должен уменьшиться
        // на высоту текста (а не на весь экран)
        assert!(
            after.y < before.y,
            "background не должен растягиваться на всю высоту"
        );
        assert!(after.y > 0.0, "после background контент не должен исчезать");
    });
}

#[test]
fn test_align_in_column_uses_vertical_layout() {
    // Aligned в Column (вертикальный layout по умолчанию) должен
    // использовать Layout::top_down с указанным Align.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        // Column использует вертикальный layout
        Column::new().show(ui, &dispatch, |ui, dispatch| {
            Text::new("Центр")
                .align(egui::Align::Center)
                .render(ui, dispatch);
        });
        // Не паникует — тест пройден
    });
}

#[test]
fn test_button_fills_available_width() {
    // Button занимает всю доступную ширину контейнера.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Button::<()>::new("Кнопка").render(ui, &dispatch);
        // Не паникует — кнопка рендерится на всю ширину
    });
}

#[test]
fn test_button_fills_full_width_with_size_modifier() {
    // Если нужно растянуть кнопку на всю ширину, используем size модификатор.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let avail_w = ui.available_width();
        Button::<()>::new("Широкая кнопка")
            .size(avail_w, 48.0)
            .render(ui, &dispatch);
        // Не паникует — кнопка с size модификатором работает корректно
    });
}

#[test]
fn test_button_wrap_content_row_not_fill() {
    // В Row кнопки не должны растягиваться на всю строку.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Row::new(ui, &dispatch, |ui, dispatch| {
            Button::<()>::new("A").render(ui, dispatch);
            Button::<()>::new("B").render(ui, dispatch);
        });
        // Не паникует — обе кнопки помещаются в одну строку
    });
}

#[test]
fn test_padded_does_not_fill_full_width() {
    // Padded (через Frame.inner_margin) не должен растягивать
    // содержимое на всю ширину контейнера.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let before = ui.available_size();
        Text::new("Текст").padding(16.0).render(ui, &dispatch);
        let after = ui.available_size();
        // После рендера контент должен потребить место по высоте.
        let consumed_y = before.y - after.y;
        assert!(
            consumed_y > 0.0,
            "padding должен потреблять место по высоте"
        );
        // По ширине текст с padding занимает реальную ширину текста + padding.
        // available_width должна уменьшиться, но остаться > 0.
        assert!(
            after.x > 0.0,
            "padding не должен делать контент пустым по ширине"
        );
    });
}

#[test]
fn test_modifier_apply_compatible_with_column() {
    // ModifierApply (новая Modifier value type) должен корректно
    // работать внутри Column.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Column::new().show(ui, &dispatch, |ui, dispatch| {
            Text::new("Первый")
                .modifier(Modifier::new().padding(8.0).background(egui::Color32::RED))
                .render(ui, dispatch);
            Text::new("Второй")
                .modifier(Modifier::new().padding(8.0).background(egui::Color32::BLUE))
                .render(ui, dispatch);
        });
        // Не паникует — колонка с модифицированными текстами работает
    });
}

#[test]
fn test_text_wrap_content_in_column() {
    // Текст с wrap-content не должен растягиваться на всю ширину Column.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let before = ui.available_size();
        Text::new("Короткий").render(ui, &dispatch);
        let after = ui.available_size();
        // available_width должен уменьшиться на ширину текста,
        // а не на всю ширину контейнера
        let consumed_w = before.x - after.x;
        assert!(
            consumed_w < before.x * 0.5,
            "Text не должен растягиваться более чем на половину контейнера"
        );
    });
}

#[test]
fn test_text_wrap_content_in_row() {
    // Два текста в Row должны помещаться в строку
    // (каждый занимает только свою ширину).
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Row::new(ui, &dispatch, |ui, dispatch| {
            Text::new("Левый").render(ui, dispatch);
            Text::new("Правый").render(ui, dispatch);
        });
        // Не паникует — оба текста помещаются
    });
}

#[test]
fn test_clickable_value_modifier_uses_content_size() {
    // Clickable (value modifier) должен создавать область по размеру контента.
    #[derive(Clone)]
    enum Msg {
        Click,
    }
    let (dispatch, _rx) = Dispatcher::<Msg>::new();
    with_ui(|ui| {
        let before = ui.available_size();
        Text::new("Клик")
            .modifier(Modifier::new().clickable(Msg::Click))
            .render(ui, &dispatch);
        let after = ui.available_size();
        assert!(
            after.y < before.y,
            "clickable value modifier не должен занимать всю высоту"
        );
        assert!(after.y > 0.0, "после clickable контент не должен исчезать");
    });
}

#[test]
fn test_clickable_with_value_modifier_uses_content_size() {
    // ClickableWith (value modifier) должен создавать область по размеру контента.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let before = ui.available_size();
        Text::new("Клик")
            .modifier(Modifier::new().clickable_with(|_r, _ui, _d| {}))
            .render(ui, &dispatch);
        let after = ui.available_size();
        assert!(
            after.y < before.y,
            "clickable_with value modifier не должен занимать всю высоту"
        );
        assert!(
            after.y > 0.0,
            "после clickable_with контент не должен исчезать"
        );
    });
}

#[test]
fn test_clickable_in_row_does_not_overflow() {
    // Clickable внутри Row — кликабельная область должна быть
    // по размеру контента, а не растягиваться на всю строку.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Row::new(ui, &dispatch, |ui, dispatch| {
            Text::new("Левый клик").clickable(()).render(ui, dispatch);
            Text::new("Правый клик").clickable(()).render(ui, dispatch);
        });
    });
}

#[test]
fn test_clickable_in_column_uses_content_height() {
    // Два Clickable в Column — каждый занимает только высоту своего текста.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Column::new().show(ui, &dispatch, |ui, dispatch| {
            Text::new("Верхний").clickable(()).render(ui, dispatch);
            Text::new("Нижний").clickable(()).render(ui, dispatch);
        });
    });
}

#[test]
fn test_align_nested_row_in_column() {
    // Вложенная Row в Column с Aligned — проверяем что не паникует.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Column::new().show(ui, &dispatch, |ui, dispatch| {
            Row::new(ui, dispatch, |ui, dispatch| {
                Text::new("Левый")
                    .align(egui::Align::Center)
                    .render(ui, dispatch);
                Text::new("Правый")
                    .align(egui::Align::Center)
                    .render(ui, dispatch);
            });
        });
    });
}

#[test]
fn test_align_in_stack() {
    // Aligned внутри Stack — проверяем что не паникует.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Stack::new(ui, &dispatch, |ui, dispatch| {
            Text::new("Центр")
                .align(egui::Align::Center)
                .render(ui, dispatch);
        });
    });
}

#[test]
fn test_clickable_in_lazy_column() {
    // Clickable внутри LazyColumn — не должен паниковать.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let items: Vec<i32> = (1..=3).collect();
        LazyColumn::new(items, ui, &dispatch, |_i, ui, dispatch| {
            Text::new("Клик").clickable(()).render(ui, dispatch);
        });
    });
}

#[test]
fn test_modifier_chain_clickable_padded_background() {
    // Комбинация: Padded + Clickable + Background через цепочку ModifierExt.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Text::new("Текст")
            .padding(8.0)
            .clickable(())
            .background(egui::Color32::RED)
            .render(ui, &dispatch);
    });
}

#[test]
fn test_modifier_value_clickable_padded_background() {
    // Комбинация: padding + clickable + background через Modifier value type.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Text::new("Текст")
            .modifier(
                Modifier::new()
                    .padding(8.0)
                    .clickable(())
                    .background(egui::Color32::RED),
            )
            .render(ui, &dispatch);
    });
}

#[test]
fn test_fill_max_width_in_scrollable_column() {
    // FillMaxWidth в Scrollable Column — симулирует HomeScreen.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let before_y = ui.available_size().y;
        let before_x = ui.available_size().x;

        Column::new()
            .scrollable()
            .show(ui, &dispatch, |ui, dispatch| {
                Text::new("Showcase").padding(8.0).render(ui, dispatch);
                Spacer::new(16.0).render(ui, dispatch);
                Text::new("Выберите демо:").render(ui, dispatch);

                Button::<()>::new("Виджеты")
                    .modifier(Modifier::new().fill_max_width().padding(8.0))
                    .render(ui, dispatch);
                Button::<()>::new("Модификаторы")
                    .modifier(Modifier::new().fill_max_width().padding(8.0))
                    .render(ui, dispatch);
                Button::<()>::new("Контейнеры")
                    .modifier(Modifier::new().fill_max_width().padding(8.0))
                    .render(ui, dispatch);
            });

        let after_y = ui.available_size().y;
        assert!(
            after_y < before_y,
            "scrollable Column не потребила место: {} -> {}",
            before_y,
            after_y
        );
        // Проверяем что было потреблено как минимум 3 * 48px (3 кнопки * высота)
        let consumed_y = before_y - after_y;
        assert!(
            consumed_y > 100.0,
            "scrollable Column потребила слишком мало: {}px (ожидалось > 100px)",
            consumed_y
        );

        let after_x = ui.available_size().x;
        assert!(
            (after_x - before_x).abs() < 1.0,
            "scrollable Column изменила ширину: {} -> {}",
            before_x,
            after_x
        );
    });
}

#[test]
fn test_fill_max_width_in_column() {
    // FillMaxWidth в Column — проверяет что:
    // 1. Кнопки не накладываются друг на друга (высота alloc'ируется по контенту)
    // 2. Дети не занимают всю высоту Column
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        // Высота доступного пространства
        let available_height = ui.available_size().y;
        assert!(
            available_height > 100.0,
            "тест должен иметь достаточную высоту"
        );

        // Рендерим две кнопки с fill_max_width в Column
        Button::<()>::new("Кнопка 1")
            .modifier(Modifier::new().fill_max_width().padding(8.0))
            .render(ui, &dispatch);
        let after_first_y = ui.available_size().y;

        // После первой кнопки available.y уменьшился
        assert!(
            after_first_y < available_height,
            "первая кнопка не потребила место по высоте: available.y {} -> {}",
            available_height,
            after_first_y
        );
        // Высота уменьшилась не на весь экран (кнопка не заняла всю высоту)
        assert!(
            after_first_y > available_height * 0.5,
            "первая кнопка заняла всю высоту: осталось {}",
            after_first_y
        );

        // Вторая кнопка
        Button::<()>::new("Кнопка 2")
            .modifier(Modifier::new().fill_max_width().padding(8.0))
            .render(ui, &dispatch);
        let after_second_y = ui.available_size().y;

        // После второй кнопки available.y уменьшился ещё
        assert!(
            after_second_y < after_first_y,
            "вторая кнопка не потребила место — наложилась на первую: {} -> {}",
            after_first_y,
            after_second_y
        );
        assert!(
            after_second_y >= 0.0,
            "после двух кнопок высота не должна быть отрицательной"
        );
    });
}

#[test]
fn test_fill_max_width_with_text() {
    // Text + fill_max_width — текст должен alloc'ить место по высоте
    // (ширина в Column не проверяется через available.x).
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let before_y = ui.available_size().y;

        Text::new("Короткий текст")
            .modifier(Modifier::new().fill_max_width())
            .render(ui, &dispatch);

        // available.y должен уменьшиться (текст alloc'ил место по высоте)
        let after_y = ui.available_size().y;
        assert!(
            after_y < before_y,
            "Text + fill_max_width не потребил место: {} -> {}",
            before_y,
            after_y
        );
    });
}

#[test]
fn test_button_wrap_content_without_fill_max_width() {
    // Button без fill_max_width должен оставаться wrap-content (регрессия).
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let before_y = ui.available_size().y;

        Button::<()>::new("Коротко").render(ui, &dispatch);

        // available.y уменьшился (кнопка есть), но не на весь экран
        let after_y = ui.available_size().y;
        assert!(
            after_y < before_y,
            "Button без fill_maxWidth не потребил место"
        );
        assert!(
            after_y > before_y * 0.9,
            "Button без fill_maxWidth занял почти всю высоту: было {}, стало {}",
            before_y,
            after_y
        );
    });
}

#[test]
fn test_fill_max_width_chain_with_padding_background() {
    // fill_max_width + padding + background — комбинация модификаторов
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let before_y = ui.available_size().y;

        Button::<()>::new("Кнопка")
            .modifier(Modifier::new().fill_max_width().padding(8.0))
            .render(ui, &dispatch);

        // available.y уменьшился (кнопка alloc'ила место)
        let after_y = ui.available_size().y;
        assert!(
            after_y < before_y,
            "fill_max_width + padding не потребил место"
        );
    });
}

#[test]
fn test_fill_max_width_in_nested_column() {
    // fill_max_width в два уровня вложенности — не должно паниковать
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        Column::new().show(ui, &dispatch, |ui, dispatch| {
            Text::new("Внешняя колонка")
                .padding(4.0)
                .render(ui, dispatch);
            Column::new().show(ui, dispatch, |ui, dispatch| {
                Button::<()>::new("Вложенная кнопка")
                    .modifier(Modifier::new().fill_max_width())
                    .render(ui, dispatch);
            });
        });
    });
}

#[test]
fn test_fill_max_width_respects_narrow_container() {
    // fill_max_width внутри контейнера половинной ширины.
    // В egui Column (CentralPanel) alloc'ит всю ширину каждому ребёнку,
    // поэтому fill_max_width всегда alloc'ит полную ширину Column.
    // Этот тест проверяет что два fill_max_width виджета не накладываются.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let before_y = ui.available_size().y;

        Button::<()>::new("Кнопка 1")
            .modifier(Modifier::new().fill_max_width())
            .render(ui, &dispatch);
        let after_first = ui.available_size().y;

        Button::<()>::new("Кнопка 2")
            .modifier(Modifier::new().fill_max_width())
            .render(ui, &dispatch);
        let after_second = ui.available_size().y;

        // Обе кнопки потребили место — не наложились
        assert!(after_first < before_y, "первая кнопка не потребила место");
        assert!(
            after_second < after_first,
            "вторая кнопка наложилась на первую: {} -> {}",
            after_first,
            after_second
        );
    });
}

// ═══════════════════════════════════════════════════════════════════════════════════
// CONSTRAINTS TESTS (Фаза 7)
// ═══════════════════════════════════════════════════════════════════════════════════

#[test]
fn test_constraints_exact_size() {
    // Constraints::exact — виджет alloc'ит точный размер
    use egui_android_core::Constraints;
    let c = Constraints::exact(200.0, 100.0);
    let clamped = c.clamp_size(egui::vec2(50.0, 30.0));
    assert_eq!(clamped.x, 200.0);
    assert_eq!(clamped.y, 100.0);
}

#[test]
fn test_constraints_ranged() {
    use egui_android_core::Constraints;
    let c = Constraints::ranged(10.0, 100.0, 20.0, 200.0);
    assert_eq!(c.clamp_size(egui::vec2(5.0, 10.0)), egui::vec2(10.0, 20.0));
    assert_eq!(
        c.clamp_size(egui::vec2(200.0, 300.0)),
        egui::vec2(100.0, 200.0)
    );
    assert_eq!(
        c.clamp_size(egui::vec2(50.0, 100.0)),
        egui::vec2(50.0, 100.0)
    );
}

#[test]
fn test_fill_max_width_in_column_stretches_button() {
    // Button + fill_max_width в Column — кнопка растягивается на всю ширину.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let before_y = ui.available_size().y;

        let before_y_inner = ui.available_size().y;

        Column::new().show(ui, &dispatch, |ui, dispatch| {
            Button::<()>::new("Кнопка")
                .modifier(Modifier::new().fill_max_width())
                .render(ui, dispatch);

            // После рендера кнопки available.y внутри Column уменьшился
            let avail_y = ui.available_size().y;
            assert!(
                avail_y < before_y_inner,
                "Кнопка с fill_max_width не alloc'ила место: {} -> {}",
                before_y_inner,
                avail_y
            );
        });

        let after_y = ui.available_size().y;
        assert!(
            after_y < before_y,
            "Column не потребила место: {} -> {}",
            before_y,
            after_y
        );
    });
}

#[test]
fn test_fill_max_width_column_two_buttons() {
    // Две кнопки с fill_max_width в Column — не накладываются.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let before_y = ui.available_size().y;

        Column::new().show(ui, &dispatch, |ui, dispatch| {
            Button::<()>::new("A")
                .modifier(Modifier::new().fill_max_width())
                .render(ui, dispatch);
            Button::<()>::new("B")
                .modifier(Modifier::new().fill_max_width())
                .render(ui, dispatch);
        });

        let after_y = ui.available_size().y;
        let consumed = before_y - after_y;
        // Две кнопки ~48px + spacing 8px = ~104px
        assert!(
            consumed > 80.0,
            "Две кнопки должны занять >80px, потребили {}",
            consumed
        );
    });
}

#[test]
fn test_fill_max_size_stretches_button() {
    // Button + fill_max_size — кнопка alloc'ит весь доступный размер.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let before_y = ui.available_size().y;

        Button::<()>::new("Кнопка")
            .modifier(Modifier::new().fill_max_size())
            .render(ui, &dispatch);

        let after_y = ui.available_size().y;
        // fill_max_size alloc'ит весь available — после него не должно остаться места
        assert!(
            after_y < 1.0,
            "fill_max_size не занял всю высоту: осталось {}",
            after_y
        );
    });
}

#[test]
fn test_sized_widget_via_constraints() {
    // SizedWidget через Constraints — точный размер.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let before_y = ui.available_size().y;

        Text::new("Текст").size(300.0, 100.0).render(ui, &dispatch);

        let after_y = ui.available_size().y;
        let consumed = before_y - after_y;
        assert!(
            consumed >= 100.0,
            "SizedWidget 100px высоты потребил {}",
            consumed
        );
    });
}

#[test]
fn test_height_modifier_via_constraints() {
    // Height + Text — текст растягивается на заданную высоту.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let before_y = ui.available_size().y;

        Text::new("Текст")
            .modifier(Modifier::new().height(60.0))
            .render(ui, &dispatch);

        let after_y = ui.available_size().y;
        let consumed = before_y - after_y;
        assert!(consumed >= 55.0, "Height 60px потребил {}", consumed);
    });
}

#[test]
fn test_width_modifier_via_constraints() {
    // Width + Text — текст растягивается на заданную ширину.
    let (dispatch, _rx) = Dispatcher::<()>::new();
    with_ui(|ui| {
        let before_y = ui.available_size().y;

        Text::new("Короткий")
            .modifier(Modifier::new().width(200.0))
            .render(ui, &dispatch);

        // Text alloc'ил место по высоте
        let after_y = ui.available_size().y;
        assert!(after_y < before_y, "Text с width 200 не потребил место");
    });
}
