//! ModifierValueScreen — демонстрация новой Modifier value type системы.
//!
//! Показывает все возможности Modifier<M>: padding, padding_hv, padding_edges,
//! fill_max_width, fill_max_size, wrap_content_width, wrap_content_size,
//! width, height, width_in, height_in, background, border, rounding, alpha,
//! clip, shadow, clickable, clickable_with и их комбинации.

use egui::Color32;
use egui_android_framework::{
    runtime::Dispatcher,
    ui::{
        containers::Column,
        modifier::{legacy::ModifierExt, Modifier, ModifierApply},
        remember,
        widgets::{Button, Spacer, Text, Widget},
        UiWrapper,
    },
};

use crate::root_component::RootMsg;

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

                // 1. padding — одинаковый со всех сторон
                Text::new("1) padding(all) — отступ со всех сторон").render(ui, dispatch);
                Text::new("Текст с padding 16")
                    .modifier(Modifier::new().padding(16.0).background(Color32::DARK_GRAY))
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // 2. padding_hv — горизонтальный и вертикальный
                Text::new("2) padding_hv(h, v) — гор 16, вер 8").render(ui, dispatch);
                Text::new("Горизонтальный и вертикальный padding")
                    .modifier(
                        Modifier::new()
                            .padding_hv(16.0, 8.0)
                            .background(Color32::from_rgb(80, 40, 20)),
                    )
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // 3. padding_edges — по сторонам
                Text::new("3) padding_edges(l, t, r, b) — 8, 16, 4, 2").render(ui, dispatch);
                Text::new("Отступы по сторонам")
                    .modifier(
                        Modifier::new()
                            .padding_edges(8.0, 16.0, 4.0, 2.0)
                            .background(Color32::from_rgb(60, 30, 60)),
                    )
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // ═══ Size constraints ══════════════════════════════════

                // 4. fill_max_width — на всю ширину
                Text::new("4) fill_max_width() — на всю ширину родителя").render(ui, dispatch);
                Text::new("На всю ширину")
                    .modifier(
                        Modifier::new()
                            .fill_max_width()
                            .background(Color32::from_rgb(20, 40, 80)),
                    )
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // 5. fill_max_size — всё доступное пространство
                Text::new("5) fill_max_size() — всё доступное пространство").render(ui, dispatch);
                Text::new("Весь экран")
                    .modifier(
                        Modifier::new()
                            .fill_max_size()
                            .background(Color32::from_rgb(30, 30, 30)),
                    )
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // 6. Fixed width + height
                Text::new("6) width(w) + height(h) — фиксированный размер").render(ui, dispatch);
                Text::new("200x48")
                    .modifier(
                        Modifier::new()
                            .width(200.0)
                            .height(48.0)
                            .background(Color32::DARK_GREEN),
                    )
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // 7. width_in + height_in — ограничения
                Text::new("7) width_in(min, max) + height_in(min, max)").render(ui, dispatch);
                Text::new("Ширина 100..300, высота 32..64")
                    .modifier(
                        Modifier::new()
                            .width_in(100.0, 300.0)
                            .height_in(32.0, 64.0)
                            .background(Color32::from_rgb(40, 80, 40)),
                    )
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // ═══ Wrap content ═════════════════════════════════════

                // 8. wrap_content_width
                Text::new("8) wrap_content_width() — ширина по содержимому").render(ui, dispatch);
                Text::new("Короткий текст (wrap_content_width)")
                    .modifier(
                        Modifier::new()
                            .wrap_content_width()
                            .background(Color32::RED),
                    )
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // 9. wrap_content_size
                Text::new("9) wrap_content_size() — размер по содержимому").render(ui, dispatch);
                Text::new("Короткий (wrap_content_size)")
                    .modifier(
                        Modifier::new()
                            .wrap_content_size()
                            .background(Color32::BLUE),
                    )
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // ═══ Appearance ════════════════════════════════════════

                // 10. background
                Text::new("10) background(color) — цвет фона").render(ui, dispatch);
                Text::new("Синий фон")
                    .modifier(
                        Modifier::new()
                            .background(Color32::from_rgb(0, 80, 200))
                            .padding(8.0),
                    )
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // 11. border + rounding
                Text::new("11) border(w, color) + rounding(radius)").render(ui, dispatch);
                Text::new("Рамка 2px, скругление 8px")
                    .modifier(
                        Modifier::new()
                            .padding(12.0)
                            .border(2.0, Color32::WHITE)
                            .rounding(8.0),
                    )
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // 12. alpha
                Text::new("12) alpha(a) — прозрачность 0.5").render(ui, dispatch);
                Text::new("Полупрозрачный текст")
                    .modifier(
                        Modifier::new()
                            .padding(8.0)
                            .background(Color32::BLUE)
                            .alpha(0.5),
                    )
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // 13. clip
                Text::new("13) clip(CornerRadius) — обрезка по скруглению").render(ui, dispatch);
                Text::new("Длинный текст, который будет обрезан по скруглению")
                    .modifier(
                        Modifier::new()
                            .clip(egui::CornerRadius::same(8))
                            .padding(8.0)
                            .background(Color32::from_rgb(40, 80, 40)),
                    )
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // 14. shadow
                Text::new("14) shadow(elevation) — тень").render(ui, dispatch);
                Text::new("С тенью (elevation 4)")
                    .modifier(
                        Modifier::new()
                            .padding(16.0)
                            .shadow(4.0)
                            .background(Color32::WHITE),
                    )
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // ═══ Interaction ═══════════════════════════════════════

                // 15. clickable
                Text::new("15) clickable(msg) — клик → dispatch").render(ui, dispatch);
                Text::new("Нажми меня (только текст!) — назад")
                    .modifier(
                        Modifier::new()
                            .padding(8.0)
                            .background(Color32::from_rgb(60, 40, 80))
                            .clickable(RootMsg::Back),
                    )
                    .render(ui, dispatch);

                Spacer::new(8.0).render(ui, dispatch);

                // 16. clickable_with
                Text::new("16) clickable_with(closure) — клик → closure").render(ui, dispatch);
                let counter = remember(ui, "mv_counter", || 0i32);
                Text::new(format!("Счётчик кликов: {}", counter.get()))
                    .padding(8.0)
                    .background(Color32::from_gray(50))
                    .render(ui, dispatch);

                Text::new("+1 (локально, через clickable_with)")
                    .modifier(
                        Modifier::new()
                            .padding(8.0)
                            .background(Color32::from_rgb(40, 60, 40))
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

                // 17. Комбинация всего
                Text::new("17) Комбинация всех модификаторов").render(ui, dispatch);
                Text::new("Всё вместе")
                    .modifier(
                        Modifier::new()
                            .fill_max_width()
                            .padding(16.0)
                            .background(Color32::from_rgb(40, 40, 80))
                            .rounding(12.0)
                            .border(1.0, Color32::GRAY)
                            .alpha(0.9),
                    )
                    .render(ui, dispatch);

                Spacer::new(16.0).render(ui, dispatch);

                // Кнопка назад
                Button::new("← Назад")
                    .on_click(RootMsg::Back)
                    .modifier(Modifier::new().fill_max_width().padding(8.0))
                    .render(ui, dispatch);
            });
    }
}
