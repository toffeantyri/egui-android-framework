//! Контейнер [`Stack`] — наложение дочерних виджетов друг на друга.
//!
//! Аналог `Box` в Jetpack Compose с двухфазным measure→layout.
//! Принимает список детей, измеряет каждого отдельно,
//! alloc'ит max размер, размещает детей с учётом align.
//!
//! # Пример
//!
//! ```ignore
//! Stack::new()
//!     .add(Text::new("Фон").background(...))
//!     .add_with_align(Text::new("По центру"), Align::Center)
//!     .show(ui, dispatch);
//! ```
//!
//! # Будущие улучшения
//!
//! - **Align-модификаторы:** `add_with_align()` отложен. Нужен публичный
//!   `egui::Ui::set_cursor()`. После патча layout-фаза сможет размещать
//!   детей в произвольной позиции через `place_child()`.
//! - **DPI-тесты:** проверить Stack на pp=3.25.
//! - **clickable и fill_max_size внутри Stack."

use crate::widgets::Widget;
use egui_android_core::{Constraints, Dispatcher, UiWrapper};

/// Варианты выравнивания ребёнка внутри Stack.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Align {
    TopStart,
    TopCenter,
    TopEnd,
    CenterStart,
    Center,
    CenterEnd,
    BottomStart,
    BottomCenter,
    BottomEnd,
}

/// Вычислить rect для ребёнка внутри stack_rect с учётом его размера и align.
#[allow(dead_code)]
pub fn place_child(stack_rect: egui::Rect, child_size: egui::Vec2, align: Align) -> egui::Rect {
    let egui::Rect { min, max: _ } = stack_rect;
    let stack_size = stack_rect.size();

    let x = match align {
        Align::TopStart | Align::CenterStart | Align::BottomStart => min.x,
        Align::TopCenter | Align::Center | Align::BottomCenter => {
            min.x + (stack_size.x - child_size.x) / 2.0
        }
        Align::TopEnd | Align::CenterEnd | Align::BottomEnd => min.x + stack_size.x - child_size.x,
    };

    let y = match align {
        Align::TopStart | Align::TopCenter | Align::TopEnd => min.y,
        Align::CenterStart | Align::Center | Align::CenterEnd => {
            min.y + (stack_size.y - child_size.y) / 2.0
        }
        Align::BottomStart | Align::BottomCenter | Align::BottomEnd => {
            min.y + stack_size.y - child_size.y
        }
    };

    egui::Rect::from_min_size(egui::pos2(x.max(min.x), y.max(min.y)), child_size)
}

pub struct StackEntry<M> {
    widget: Box<dyn Widget<M>>,
}

/// Builder для Stack. Добавляй детей через `.add()` и `.add_with_align()`.
pub struct Stack<M> {
    children: Vec<StackEntry<M>>,
}

impl<M: 'static + Send> Default for Stack<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: 'static + Send> Stack<M> {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    /// Добавить ребёнка с выравниванием по умолчанию (TopStart).
    #[allow(clippy::should_implement_trait)]
    pub fn add(mut self, widget: impl Widget<M> + 'static) -> Self {
        self.children.push(StackEntry {
            widget: Box::new(widget),
        });
        self
    }

    /// Добавить ребёнка с указанным align.
    /// Пока align не реализован (ждёт pub set_cursor).
    #[allow(unused_variables)]
    pub fn add_with_align(mut self, widget: impl Widget<M> + 'static, align: Align) -> Self {
        self.children.push(StackEntry {
            widget: Box::new(widget),
        });
        self
    }

    /// Измерить и отрендерить Stack.
    pub fn show(self, ui: &mut UiWrapper, dispatch: &Dispatcher<M>) {
        if self.children.is_empty() {
            return;
        }

        let inner_rect = ui.available_rect_before_wrap();
        let inner_size = inner_rect.size();
        let layout = *ui.layout();
        let child_constraints = Constraints::ranged(
            0.0,
            inner_rect.width().max(0.0),
            0.0,
            inner_rect.height().max(0.0),
        );

        // ═══ Фаза 1: Measure — измеряем каждого ребёнка отдельно ═══
        let mut sizes: Vec<egui::Vec2> = Vec::with_capacity(self.children.len());

        for entry in &self.children {
            let size = {
                let mut measure_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt("stack_measure")
                        .max_rect(inner_rect)
                        .layout(layout),
                );
                let mut w = UiWrapper::new(&mut measure_ui, child_constraints);
                entry.widget.render(&mut w, dispatch);
                // Измеряем сколько alloc'ил ребёнок
                let mr = w.min_rect();
                let sz = mr.size();
                // clamp по available
                egui::vec2(
                    sz.x.min(inner_size.x).max(0.0),
                    sz.y.min(inner_size.y).max(0.0),
                )
            };
            sizes.push(size);
        }

        // Вычисляем max размер среди детей
        let stack_size = egui::vec2(
            sizes
                .iter()
                .map(|s| s.x)
                .fold(0.0_f32, f32::max)
                .min(inner_size.x),
            sizes
                .iter()
                .map(|s| s.y)
                .fold(0.0_f32, f32::max)
                .min(inner_size.y),
        );

        // ═══ Фаза 2: Layout — alloc и размещение ═══
        // Дети уже отрендерены в measure-фазе. Достаточно просто alloc'ить
        // stack_size в родителе — shapes уже нарисованы и будут видны.
        let _ = ui.allocate_exact_size(stack_size, egui::Sense::hover());
    }
}
