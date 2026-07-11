//! ModifierValueScreen — демонстрация новой Modifier value type системы.
//!
//! Показывает все возможности Modifier<M>: padding, padding_hv, padding_edges,
//! fill_max_width, fill_max_size, wrap_content_width, wrap_content_size,
//! width, height, width_in, height_in, background, border, rounding, alpha,
//! clip, shadow, clickable, clickable_with и их комбинации.
//!
//! Каждый пример показывает внешнюю рамкой границу области модификатора,
//! чтобы было наглядно видно отступы (padding) и размер контента.

use egui::Color32;
use egui_android_framework::{
    runtime::Dispatcher,
    ui::{
        containers::Column,
        modifier::{Modifier, ModifierDsl},
        remember,
        theme::Colors,
        widgets::{Button, Spacer, Text, Widget},
        UiWrapper,
    },
};

use crate::root_component::RootMsg;

/// Показать пример с рамкой: border вокруг модификатора для наглядности.
fn show_example(
    ui: &mut UiWrapper,
    dispatch: &Dispatcher<RootMsg>,
    title: &str,
    content: &str,
    modifier: Modifier<RootMsg>,
) {
    Text::new(title).render(ui, dispatch);
    Text::new(content)
        .modifier(
            Modifier::new().border(1.0, Color32::WHITE).then(modifier), // border снаружи, modifier внутри
        )
        .render(ui, dispatch);
    Spacer::new(8.0).render(ui, dispatch);
}

/// Показать пример с рамкой и кастомным фоном.
/// Модификаторы применяются как в Compose: первый — внешний, последний — внутренний.
/// Чтобы padding был внутри background, modifier добавляется ПОСЛЕ background.
fn show_example_bg(
    ui: &mut UiWrapper,
    dispatch: &Dispatcher<RootMsg>,
    title: &str,
    content: &str,
    modifier: Modifier<RootMsg>,
    bg: Color32,
) {
    Text::new(title).render(ui, dispatch);
    Text::new(content)
        .modifier(
            Modifier::new()
                .background(bg)
                .border(2.0, Colors::LIGHT_GREEN)
                .then(modifier),
        )
        .render(ui, dispatch);
    Spacer::new(8.0).render(ui, dispatch);
}

/// Экран демонстрации новой Modifier системы.
pub struct ModifierValueScreen;

impl ModifierValueScreen {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<RootMsg>) {
        Column::new()
            .scrollable()
            .show(ui, dispatch, |ui, dispatch| {
                Text::new("Новая Modifier Value Type")
                    .modifier(Modifier::new().padding(8.0))
                    .render(ui, dispatch);
                Spacer::new(8.0).render(ui, dispatch);

                // ═══ Padding ═══════════════════════════════════════════

                show_example_bg(
                    ui,
                    dispatch,
                    "1) padding(all) — отступ со всех сторон",
                    "Текст с padding 16",
                    Modifier::new().padding(16.0).wrap_content_size(),
                    Color32::from_rgb(60, 60, 80),
                );

                show_example_bg(
                    ui,
                    dispatch,
                    "2) padding_hv(h, v) — гор 16, вер 8",
                    "Горизонтальный и вертикальный padding",
                    Modifier::new().padding_hv(16.0, 8.0).wrap_content_size(),
                    Color32::from_rgb(80, 40, 20),
                );

                show_example_bg(
                    ui,
                    dispatch,
                    "3) padding_edges(l, t, r, b) — 8, 16, 4, 2",
                    "Отступы по сторонам",
                    Modifier::new()
                        .padding_edges(8.0, 16.0, 4.0, 2.0)
                        .wrap_content_size(),
                    Color32::from_rgb(60, 30, 60),
                );

                // ═══ Size constraints ══════════════════════════════════

                show_example_bg(
                    ui,
                    dispatch,
                    "4) fill_max_width() — на всю ширину родителя",
                    "На всю ширину",
                    Modifier::new().fill_max_width(),
                    Color32::from_rgb(20, 40, 80),
                );

                show_example_bg(
                    ui,
                    dispatch,
                    "5) fill_max_size() — всё доступное пространство",
                    "Весь экран",
                    Modifier::new().fill_max_size(),
                    Color32::from_rgb(30, 30, 30),
                );

                show_example_bg(
                    ui,
                    dispatch,
                    "6) width(w) + height(h) — фиксированный размер",
                    "200x48",
                    Modifier::new().width(200.0).height(48.0),
                    Color32::DARK_GREEN,
                );

                show_example_bg(
                    ui,
                    dispatch,
                    "7) width_in(min, max) + height_in(min, max)",
                    "Ширина 100..300, высота 32..64",
                    Modifier::new().width_in(100.0, 300.0).height_in(32.0, 64.0),
                    Color32::from_rgb(40, 80, 40),
                );

                // ═══ Wrap content ═════════════════════════════════════

                show_example_bg(
                    ui,
                    dispatch,
                    "8) wrap_content_width() — ширина по содержимому",
                    "Короткий текст (wrap_content_width)",
                    Modifier::new().wrap_content_width(),
                    Color32::RED,
                );

                show_example_bg(
                    ui,
                    dispatch,
                    "9) wrap_content_size() — размер по содержимому",
                    "Короткий (wrap_content_size)",
                    Modifier::new().wrap_content_size(),
                    Color32::BLUE,
                );

                // ═══ Appearance ════════════════════════════════════════

                show_example_bg(
                    ui,
                    dispatch,
                    "10) background(color) — цвет фона",
                    "Синий фон",
                    Modifier::new().padding(8.0),
                    Color32::from_rgb(0, 80, 200),
                );

                show_example(
                    ui,
                    dispatch,
                    "11) border(w, color) + rounding(radius)",
                    "Рамка 2px, скругление 8px",
                    Modifier::new()
                        .padding(12.0)
                        .border(2.0, Color32::WHITE)
                        .rounding(8.0),
                );

                show_example_bg(
                    ui,
                    dispatch,
                    "12) alpha(a) — прозрачность 0.5",
                    "Полупрозрачный текст",
                    Modifier::new().padding(8.0).alpha(0.5),
                    Color32::BLUE,
                );

                show_example_bg(
                    ui,
                    dispatch,
                    "13) clip(CornerRadius) — обрезка по скруглению",
                    "Длинный текст, который будет обрезан по скруглению",
                    Modifier::new()
                        .clip(egui::CornerRadius::same(8))
                        .padding(8.0),
                    Color32::from_rgb(40, 80, 40),
                );

                // 14. shadow
                Text::new("14) shadow(elevation) — тень").render(ui, dispatch);
                Text::new("С тенью (elevation 4)")
                    .modifier(
                        Modifier::new()
                            .padding(16.0)
                            .shadow(4.0)
                            .background(Color32::WHITE)
                            .border(1.0, Color32::WHITE),
                    )
                    .render(ui, dispatch);
                Spacer::new(8.0).render(ui, dispatch);

                // ═══ Interaction ═══════════════════════════════════════

                show_example_bg(
                    ui,
                    dispatch,
                    "15) clickable(msg) — клик → dispatch",
                    "Нажми меня (только текст!) — назад",
                    Modifier::new().padding(8.0).clickable(RootMsg::Back),
                    Color32::from_rgb(60, 40, 80),
                );

                Text::new("16) clickable_with(closure) — клик → closure").render(ui, dispatch);
                let counter = remember(ui, "mv_counter", || 0i32);
                Text::new(format!("Счётчик кликов: {}", counter.get()))
                    .modifier(
                        Modifier::new()
                            .padding(8.0)
                            .background(Color32::from_gray(50))
                            .border(1.0, Color32::WHITE),
                    )
                    .render(ui, dispatch);

                Text::new("+1 (локально, через clickable_with)")
                    .modifier(
                        Modifier::new()
                            .padding(8.0)
                            .background(Color32::from_rgb(40, 60, 40))
                            .border(1.0, Color32::WHITE)
                            .clickable_with({
                                let counter = counter.clone();
                                move |_response, _ui, _dispatch| {
                                    counter.modify(|c| *c += 1);
                                }
                            }),
                    )
                    .render(ui, dispatch);
                Spacer::new(8.0).render(ui, dispatch);

                // ═══ Комбинация ═══════════════════════════════════════

                show_example_bg(
                    ui,
                    dispatch,
                    "17) Комбинация всех модификаторов",
                    "Всё вместе",
                    Modifier::new()
                        .fill_max_width()
                        .padding(16.0)
                        .rounding(12.0)
                        .alpha(0.9),
                    Color32::from_rgb(40, 40, 80),
                );

                Spacer::new(16.0).render(ui, dispatch);

                // Кнопка назад
                Button::new("← Назад")
                    .on_click(RootMsg::Back)
                    .modifier(Modifier::new().fill_max_width().padding(8.0))
                    .render(ui, dispatch);
            });
    }
}
