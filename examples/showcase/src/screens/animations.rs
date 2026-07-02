//! AnimationsScreen — демонстрация анимаций.

use egui::Frame;
use egui_android_framework::runtime::Dispatcher;
use egui_android_framework::ui::remember;

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

        ui.heading("Анимации");
        ui.add_space(8.0);

        ui.label("AnimatedVisibility:");
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
        if *show_box.get() {
            ui.set_opacity(1.0);
        } else {
            ui.set_opacity(0.0);
        }
        Frame::new().inner_margin(12.0).show(ui, |ui| {
            ui.label("Появляющийся текст");
        });
        ui.set_opacity(1.0);

        ui.add_space(8.0);
        ui.label("Fade (прозрачность):");
        ui.set_opacity(0.6);
        ui.label("Текст с fade");
        ui.set_opacity(1.0);

        ui.add_space(8.0);
        ui.label("Slide:");
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
            ui.add_space(20.0);
            ui.label("Слайд вниз");
        }

        ui.add_space(16.0);
        if ui.button("Назад").clicked() {
            dispatch.dispatch(RootMsg::Back);
        }
    }
}
