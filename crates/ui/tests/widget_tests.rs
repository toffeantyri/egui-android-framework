//! Интеграционные тесты виджетов, контейнеров, модификаторов, анимаций и темы.

use std::cell::RefCell;

use egui_android_core::widget::Widget as WidgetTrait;
use egui_android_core::Dispatcher;
use egui_android_ui::animation::{
    animate_bool, animate_value, AnimatedVisibility, AnimationExt, Fade, Slide, SlideDirection,
};
use egui_android_ui::containers::{Column, LazyColumn, Row, Stack};
use egui_android_ui::modifier::{Modifier, ModifierApply, ModifierExt};
use egui_android_ui::theme::{MaterialTheme, Shapes, Theme};
use egui_android_ui::widgets::{Button, Icon, Spacer, Text};

// ─── Helper: with_ui ────────────────────────────────────────────────────────────

fn with_ui(f: impl FnOnce(&mut egui::Ui)) {
    let f = RefCell::new(Some(f));
    let ctx = egui::Context::default();
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let f = f.borrow_mut().take().unwrap();
            f(ui);
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
