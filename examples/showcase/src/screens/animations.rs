//! AnimationsScreen — демонстрация анимаций AnimatedVisibility, Fade, Slide, AnimationExt, animate_bool.

use egui_android_framework::{
    runtime::Dispatcher,
    ui::{
        animation::{AnimatedVisibility, AnimationExt, SlideDirection},
        containers::Column,
        modifier::ModifierExt,
        remember,
        widgets::{Button, Spacer, Text, Widget},
    },
};

use crate::root_component::RootMsg;

/// Экран демонстрации анимаций.
pub struct AnimationsScreen;

impl AnimationsScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<RootMsg>) {
        let mut show_box = remember(ui, "anim_show", || false);
        let mut slide_open = remember(ui, "anim_slide", || false);

        Column::<RootMsg>::empty()
            .child(Spacer::new(16.0))
            .child(Text::new("Анимации").padding(8.0))
            .child(Spacer::new(8.0))
            // AnimatedVisibility
            .child(Text::new("AnimatedVisibility:"))
            .child(
                AnimatedVisibility::new(*show_box.get(), 0.3).child(
                    Text::new("Появляющийся текст")
                        .padding(12.0)
                        .background(egui::Color32::from_gray(40)),
                ),
            )
            .child(Spacer::new(8.0))
            // Fade (прозрачность через AnimationExt)
            .child(Text::new("Fade (прозрачность):"))
            .child(Text::new("Текст с fade 0.6").fade(0.6).padding(8.0))
            .child(Spacer::new(8.0))
            // Slide через AnimationExt
            .child(Text::new("Slide:"))
            .child(
                AnimatedVisibility::new(*slide_open.get(), 0.3).child(
                    Text::new("Слайд вниз")
                        .slide(SlideDirection::Down, 20.0)
                        .padding(8.0),
                ),
            )
            .child(Spacer::new(16.0))
            .child(Button::new("← Назад").on_click(RootMsg::Back).padding(8.0))
            .render(ui, dispatch);

        // Кнопки для remember-состояния (вне Column — RememberState требует прямого доступа)
        if ui
            .button(if *show_box.get() {
                "Скрыть"
            } else {
                "Показать"
            })
            .clicked()
        {
            show_box.modify(|v| *v = !*v);
        }
        if ui
            .button(if *slide_open.get() {
                "Закрыть"
            } else {
                "Открыть"
            })
            .clicked()
        {
            slide_open.modify(|v| *v = !*v);
        }
    }
}
