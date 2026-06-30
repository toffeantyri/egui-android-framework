//! Система декларативных анимаций — аналог AnimatedVisibility / animateFloatAsState
//! из Jetpack Compose.
//!
//! Использует встроенные функции egui: `ctx.animate_bool_with_time()`
//! и `ctx.animate_value_with_time()`, которые автоматически
//! вызывают `ctx.request_repaint()` на время анимации.
//!
//! # Компоненты
//!
//! - [`AnimatedVisibility`] — плавное появление/исчезновение через Fade + Scale
//! - [`Fade`] — wrapper-виджет для прозрачности
//! - [`Slide`] — wrapper-виджет для смещения
//! - [`animate_value`] / [`animate_bool`] — хелперы интерполяции
//!
//! # Пример
//!
//! ```ignore
//! use egui_android_framework::animation::{AnimatedVisibility, AnimationExt, animate_value};
//! use std::time::Duration;
//!
//! AnimatedVisibility::new(visible, Duration::from_millis(300))
//!     .child(Text::new("Контент"))
//!     .render(ui, dispatch);
//!
//! Text::new("Полупрозрачный").fade(0.5).render(ui, dispatch);
//! ```

mod animate;
mod animated_visibility;
mod fade;
mod slide;

pub use animate::{animate_bool, animate_value};
pub use animated_visibility::AnimatedVisibility;
pub use fade::Fade;
pub use slide::{Slide, SlideDirection};

use crate::widgets::Widget;

/// Extension trait для применения анимаций к виджетам.
///
/// Автоматически реализован для всех типов, реализующих [`Widget<M>`].
pub trait AnimationExt<M>: Widget<M> + Sized {
    /// Применить прозрачность к виджету.
    ///
    /// `opacity` — значение от 0.0 (полностью прозрачный) до 1.0 (непрозрачный).
    fn fade(self, opacity: f32) -> Fade<Self, M>;

    /// Сместить виджет в заданном направлении.
    fn slide(self, direction: SlideDirection, offset: f32) -> Slide<Self, M>;
}

impl<T: Widget<M>, M> AnimationExt<M> for T {
    fn fade(self, opacity: f32) -> Fade<Self, M> {
        Fade::new(self, opacity)
    }

    fn slide(self, direction: SlideDirection, offset: f32) -> Slide<Self, M> {
        Slide::new(self, direction, offset)
    }
}
