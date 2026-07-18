//! ModifierValueScreen — демонстрация новой Modifier value type системы.
//!
//! Показывает все возможности Modifier<M>: padding, padding_hv, padding_edges,
//! fill_max_width, fill_max_size, wrap_content_width, wrap_content_size,
//! width, height, width_in, height_in, background, border, rounding, alpha,
//! clip, shadow, clickable, clickable_with и их комбинации.
//!
//! Каждый пример показывает внешней рамкой границу области модификатора,
//! чтобы было наглядно видно отступы (padding) и размер контента.
//!
//! Все цвета берутся из текущей Material Design 3 темы.

use egui::Color32;
use egui_android_framework::core::{Component as UiComponent, LifecycleObserver, UiWrapper};
use egui_android_framework::runtime::Dispatcher;
use egui_android_framework::ui::{
    containers::Column,
    modifier::{Modifier, ModifierDsl},
    remember,
    theme::{Colors, Theme},
    widgets::{Button, Spacer, Text, Widget},
};

use crate::navigation_host::RootMsg;

/// Показать пример с рамкой и фоном. Цвет текста задаётся явно для контраста.
fn show_example_bg(
    ui: &mut UiWrapper,
    dispatch: &Dispatcher<RootMsg>,
    title: &str,
    content: &str,
    modifier: Modifier<RootMsg>,
    bg: Color32,
    text_color: Color32,
) {
    Text::new(title).render(ui, dispatch);
    Text::new(content)
        .text_color(text_color)
        .modifier(
            Modifier::new()
                .background(bg)
                .border(1.0, Colors::LIGHT_GREEN)
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
}

impl LifecycleObserver for ModifierValueScreen {}

impl UiComponent for ModifierValueScreen {
    type State = ();
    type Message = RootMsg;

    fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<Self::Message>) {
        let c = &Theme::current_from_ui(ui).colors;

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
                    c.secondary,
                    c.on_secondary,
                );

                show_example_bg(
                    ui,
                    dispatch,
                    "2) padding_hv(h, v) — гор 16, вер 8",
                    "Горизонтальный и вертикальный padding",
                    Modifier::new().padding_hv(16.0, 8.0).wrap_content_size(),
                    c.secondary,
                    c.on_secondary,
                );

                show_example_bg(
                    ui,
                    dispatch,
                    "3) padding_edges(l, t, r, b) — 8, 16, 4, 2",
                    "Отступы по сторонам",
                    Modifier::new()
                        .padding_edges(8.0, 16.0, 4.0, 2.0)
                        .wrap_content_size(),
                    c.surface,
                    c.on_surface,
                );

                // ═══ Size constraints ══════════════════════════════════

                Text::new("4) fill_max_width() — на всю ширину родителя").render(ui, dispatch);
                Text::new("Слева (default)")
                    .text_color(c.on_secondary)
                    .modifier(
                        Modifier::new()
                            .fill_max_width()
                            .background(c.secondary)
                            .border(1.0, Colors::LIGHT_GREEN)
                            .padding(8.0),
                    )
                    .render(ui, dispatch);
                Text::new("По центру")
                    .align(egui::Align::Center)
                    .text_color(c.on_secondary)
                    .modifier(
                        Modifier::new()
                            .fill_max_width()
                            .background(c.secondary)
                            .border(1.0, Colors::LIGHT_GREEN)
                            .padding(8.0),
                    )
                    .render(ui, dispatch);
                Text::new("Справа")
                    .align(egui::Align::RIGHT)
                    .text_color(c.on_secondary)
                    .modifier(
                        Modifier::new()
                            .fill_max_width()
                            .background(c.secondary)
                            .border(1.0, Colors::LIGHT_GREEN)
                            .padding(8.0),
                    )
                    .render(ui, dispatch);
                Spacer::new(8.0).render(ui, dispatch);

                show_example_bg(
                    ui,
                    dispatch,
                    "5) fill_max_size() — всё доступное пространство",
                    "Весь экран",
                    Modifier::new().fill_max_size(),
                    c.surface,
                    c.on_surface,
                );

                show_example_bg(
                    ui,
                    dispatch,
                    "6) width(w) + height(h) — фиксированный размер",
                    "200x48",
                    Modifier::new().width(200.0).height(48.0),
                    c.secondary,
                    c.on_secondary,
                );

                show_example_bg(
                    ui,
                    dispatch,
                    "7) width_in(min, max) + height_in(min, max)",
                    "Ширина 100..300, высота 32..64",
                    Modifier::new().width_in(100.0, 300.0).height_in(32.0, 64.0),
                    c.secondary,
                    c.on_secondary,
                );

                // ═══ Wrap content ═════════════════════════════════════

                show_example_bg(
                    ui,
                    dispatch,
                    "8) wrap_content_width() — ширина по содержимому",
                    "Короткий текст (wrap_content_width)",
                    Modifier::new().wrap_content_width(),
                    c.primary,
                    c.on_primary,
                );

                show_example_bg(
                    ui,
                    dispatch,
                    "9) wrap_content_size() — размер по содержимому",
                    "Короткий (wrap_content_size)",
                    Modifier::new().wrap_content_size(),
                    c.primary,
                    c.on_primary,
                );

                // ═══ Appearance ════════════════════════════════════════

                show_example_bg(
                    ui,
                    dispatch,
                    "10) background(color) — цвет фона",
                    "Secondary фон",
                    Modifier::new().padding(8.0),
                    c.secondary,
                    c.on_secondary,
                );

                Text::new("11) border(w, color) + rounding(radius) — скругление 8px")
                    .render(ui, dispatch);
                Text::new("Рамка 2px со скруглением 8px")
                    .text_color(c.on_surface)
                    .modifier(
                        Modifier::new()
                            .padding(12.0)
                            .background_with_rounding(c.surface, 8.0)
                            .border_with_rounding(2.0, Colors::LIGHT_GREEN, 8.0),
                    )
                    .render(ui, dispatch);
                Spacer::new(8.0).render(ui, dispatch);

                show_example_bg(
                    ui,
                    dispatch,
                    "12) alpha(a) — прозрачность 0.5",
                    "Полупрозрачный текст",
                    Modifier::new().padding(8.0).alpha(0.5),
                    c.secondary,
                    c.on_secondary,
                );

                Text::new("13) clip(CornerRadius) — обрезка по скруглению").render(ui, dispatch);
                Text::new("Текст со скруглёнными углами фона (clip 12px)")
                    .text_color(c.on_secondary)
                    .modifier(
                        Modifier::new()
                            .clip(egui::CornerRadius::same(12))
                            .background_with_rounding(c.secondary, 12.0)
                            .padding(12.0),
                    )
                    .render(ui, dispatch);
                Spacer::new(8.0).render(ui, dispatch);

                // 14. shadow
                Text::new("14) shadow(elevation) — тень").render(ui, dispatch);
                Text::new("Блок с тенью (elevation 4)")
                    .text_color(c.on_surface)
                    .modifier(
                        Modifier::new()
                            .shadow(4.0)
                            .background_with_rounding(c.surface, 4.0),
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
                    c.primary,
                    c.on_primary,
                );

                Text::new("16) clickable_with(closure) — клик → closure").render(ui, dispatch);
                let counter = remember(ui, "mv_counter", || 0i32);
                Text::new(format!("Счётчик кликов: {}", counter.get()))
                    .text_color(c.on_surface)
                    .modifier(
                        Modifier::new()
                            .padding(8.0)
                            .background(c.surface)
                            .border(1.0, c.outline),
                    )
                    .render(ui, dispatch);

                Text::new("+1 (локально, через clickable_with)")
                    .text_color(c.on_secondary)
                    .modifier(
                        Modifier::new()
                            .padding(8.0)
                            .background(c.secondary)
                            .border(1.0, c.outline)
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
                    c.secondary,
                    c.on_secondary,
                );

                Spacer::new(16.0).render(ui, dispatch);

                // Кнопка назад
                Button::new("← Назад")
                    .on_click(RootMsg::Back)
                    .theme_colors(c.primary)
                    .text_color(c.on_primary)
                    .modifier(Modifier::new().fill_max_width().padding(8.0))
                    .render(ui, dispatch);
            });
    }

    fn handle(&mut self, _msg: Self::Message) {}

    fn state(&self) -> &Self::State {
        &()
    }
}
