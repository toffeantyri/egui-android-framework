//! AnimationsScreen — демонстрация анимаций.

use egui_android_framework::{
    animation::{AnimatedVisibility, AnimationExt, SlideDirection},
    dispatcher::Dispatcher,
    modifiers::ModifierExt,
    remember,
    widgets::{Button, Spacer, Text, Widget},
};

use crate::root_component::RootMsg;

/// Экран демонстрации анимаций.
pub struct AnimationsScreen;

impl AnimationsScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, ui: &mut egui::Ui, _dispatch: &Dispatcher<RootMsg>) {
        let mut show_box = remember(ui, "anim_show", || false);
        let mut slide_open = remember(ui, "anim_slide", || false);

        Text::new("Анимации").render(ui, _dispatch);
        Spacer::new(8.0).render(ui, _dispatch);

        Text::new("AnimatedVisibility:").render(ui, _dispatch);
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
        AnimatedVisibility::new(*show_box.get(), 0.4)
            .child(Text::new("Появляющийся текст").padding(12.0))
            .render(ui, _dispatch);

        Spacer::new(8.0).render(ui, _dispatch);
        Text::new("Fade (прозрачность):").render(ui, _dispatch);
        Text::new("Текст с fade").fade(0.6).render(ui, _dispatch);

        Spacer::new(8.0).render(ui, _dispatch);
        Text::new("Slide:").render(ui, _dispatch);
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
        if *slide_open.get() {
            Text::new("Слайд вниз")
                .slide(SlideDirection::Down, 20.0)
                .render(ui, _dispatch);
        }

        Spacer::new(16.0).render(ui, _dispatch);
        Button::new("Назад")
            .on_click(RootMsg::Back)
            .render(ui, _dispatch);
    }
}
