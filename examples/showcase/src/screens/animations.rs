//! AnimationsScreen — демонстрация анимаций AnimatedVisibility, Fade, Slide, AnimationExt.

use egui_android_framework::{
    runtime::Dispatcher,
    ui::{
        animation::{AnimatedVisibility, AnimationExt, SlideDirection},
        containers::Column,
        modifier::{Modifier, ModifierDsl},
        remember,
        theme::Theme,
        widgets::{Button, Spacer, Text, Widget},
        UiWrapper,
    },
};

use crate::root_component::RootMsg;

/// Экран демонстрации анимаций.
pub struct AnimationsScreen;

impl AnimationsScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<RootMsg>) {
        let c = &Theme::current_from_ui(ui).colors;
        let show_box = remember(ui, "anim_show", || false);
        let slide_open = remember(ui, "anim_slide", || false);

        Column::new()
            .scrollable()
            .show(ui, dispatch, |ui, dispatch| {
                Text::new("Анимации")
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);
                Spacer::new(8.0).render(ui, dispatch);

                // AnimatedVisibility
                Text::new("AnimatedVisibility:").render(ui, dispatch);
                Text::new(format!("visible = {}", *show_box.get()))
                    .text_color(c.on_secondary)
                    .modifier(Modifier::new().padding(4.0).background(c.secondary))
                    .render(ui, dispatch);

                Button::new(if *show_box.get() {
                    "Скрыть"
                } else {
                    "Показать"
                })
                .on_click_with({
                    let show_box = show_box.clone();
                    move |_ui, _dispatch| show_box.modify(|v| *v = !*v)
                })
                .theme_colors(c.secondary)
                .text_color(c.on_secondary)
                .modifier(Modifier::new().padding(8.0))
                .render(ui, dispatch);

                AnimatedVisibility::new(*show_box.get(), 0.3)
                    .child(
                        Text::new("Появляющийся текст")
                            .text_color(c.on_secondary)
                            .modifier(
                                Modifier::new()
                                    .fill_max_width()
                                    .padding(12.0)
                                    .background(c.secondary),
                            ),
                    )
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // Fade (прозрачность через AnimationExt)
                Text::new("Fade (прозрачность):").render(ui, dispatch);
                Text::new("Текст с fade 0.6")
                    .fade(0.6)
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // Slide
                Text::new("Slide:").render(ui, dispatch);
                Text::new(format!("slide_open = {}", *slide_open.get()))
                    .text_color(c.on_secondary)
                    .modifier(Modifier::new().padding(4.0).background(c.secondary))
                    .render(ui, dispatch);

                Button::new(if *slide_open.get() {
                    "Скрыть слайд"
                } else {
                    "Показать слайд"
                })
                .on_click_with({
                    let slide_open = slide_open.clone();
                    move |_ui, _dispatch| slide_open.modify(|v| *v = !*v)
                })
                .theme_colors(c.secondary)
                .text_color(c.on_secondary)
                .modifier(Modifier::new().padding(8.0))
                .render(ui, dispatch);

                AnimatedVisibility::new(*slide_open.get(), 0.3)
                    .child(
                        Text::new("Слайд вниз")
                            .slide(SlideDirection::Down, 20.0)
                            .modifier(Modifier::new().padding(8.0)),
                    )
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
}
