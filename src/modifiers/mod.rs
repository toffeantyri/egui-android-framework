//! Система модификаторов для виджетов — аналог Modifier в Jetpack Compose.
//!
//! Позволяет применять к любому виджету, реализующему [`Widget<M>`],
//! цепочку модификаторов: `.padding(16).background(Color::RED).render(ui, dispatch)`.
//!
//! Каждый модификатор — это wrapper-структура, которая сама реализует [`Widget<M>`]
//! и делегирует рендеринг внутреннему виджету, оборачивая его в дополнительный слой.
//!
//! # Порядок модификаторов важен
//!
//! `Text::new("...").padding(16).background(RED)` — фон рисуется **поверх** padding.
//! `Text::new("...").background(RED).padding(16)` — padding снаружи фона.
//!
//! # Использование
//!
//! ```ignore
//! use egui_android_framework::modifiers::ModifierExt;
//!
//! Text::new("Привет")
//!     .padding(16.0)
//!     .background(egui::Color32::from_gray(40))
//!     .render(ui, dispatch);
//! ```

mod aligned;
mod background;
mod clickable;
mod padded;
mod sized;

pub use aligned::Aligned;
pub use background::Background;
pub use clickable::Clickable;
pub use padded::Padded;
pub use sized::SizedWidget;

use crate::widgets::Widget;

/// Extension trait для применения модификаторов к виджетам.
///
/// Автоматически реализован для всех типов, реализующих [`Widget<M>`],
/// через blanket impl.
pub trait ModifierExt<M>: Widget<M> + Sized {
    /// Добавить отступы вокруг виджета.
    fn padding(self, value: f32) -> Padded<Self, M>;

    /// Установить размер виджета.
    fn size(self, width: f32, height: f32) -> SizedWidget<Self, M>;

    /// Установить цвет фона.
    fn background(self, color: egui::Color32) -> Background<Self, M>;

    /// Выровнять виджет.
    fn align(self, align: egui::Align) -> Aligned<Self, M>;

    /// Сделать виджет кликабельным.
    ///
    /// При клике диспатчит указанное сообщение.
    fn clickable(self, msg: M) -> Clickable<Self, M>
    where
        M: Clone;
}

impl<T: Widget<M>, M> ModifierExt<M> for T {
    fn padding(self, value: f32) -> Padded<Self, M> {
        Padded {
            inner: self,
            padding: value,
            _marker: std::marker::PhantomData,
        }
    }

    fn size(self, width: f32, height: f32) -> SizedWidget<Self, M> {
        SizedWidget {
            inner: self,
            width,
            height,
            _marker: std::marker::PhantomData,
        }
    }

    fn background(self, color: egui::Color32) -> Background<Self, M> {
        Background {
            inner: self,
            color,
            _marker: std::marker::PhantomData,
        }
    }

    fn align(self, align: egui::Align) -> Aligned<Self, M> {
        Aligned {
            inner: self,
            align,
            _marker: std::marker::PhantomData,
        }
    }

    fn clickable(self, msg: M) -> Clickable<Self, M>
    where
        M: Clone,
    {
        Clickable {
            inner: self,
            msg,
            _marker: std::marker::PhantomData,
        }
    }
}
