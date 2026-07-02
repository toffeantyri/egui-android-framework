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

        Column::new(ui, dispatch, |ui, dispatch| {
            Text::new("Анимации").padding(8.0).render(ui, dispatch);
            Spacer::new(8.0).render(ui, dispatch);

            // AnimatedVisibility
            Text::new("AnimatedVisibility:").render(ui, dispatch);
            AnimatedVisibility::new(*show_box.get(), 0.3)
                .child(
                    Text::new("Появляющийся текст")
                        .padding(12.0)
                        .background(egui::Color32::from_gray(40)),
                )
                .render(ui, dispatch);

            Spacer::new(8.0).render(ui, dispatch);

            // Fade (прозрачность через AnimationExt)
            Text::new("Fade (прозрачность):").render(ui, dispatch);
            Text::new("Текст с fade 0.6")
                .fade(0.6)
                .padding(8.0)
                .render(ui, dispatch);

            Spacer::new(8.0).render(ui, dispatch);

            // Slide через AnimationExt
            Text::new("Slide:").render(ui, dispatch);
            AnimatedVisibility::new(*slide_open.get(), 0.3)
                .child(
                    Text::new("Слайд вниз")
                        .slide(SlideDirection::Down, 20.0)
                        .padding(8.0),
                )
                .render(ui, dispatch);

            Spacer::new(16.0).render(ui, dispatch);
            Button::new("← Назад")
                .on_click(RootMsg::Back)
                .padding(8.0)
                .render(ui, dispatch);
        });

        // Кнопки для remember-состояния (вне Column — RememberState требует &mut)
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
