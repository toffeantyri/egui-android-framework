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
//! use egui_android_framework::ui::modifier::ModifierExt;
//!
//! Text::new("Привет")
//!     .padding(16.0)
//!     .background(egui::Color32::from_gray(40))
//!     .render(ui, dispatch);
//! ```

use std::marker::PhantomData;

use egui_android_core::{widget::Widget, Dispatcher};

// ─── Padded ────────────────────────────────────────────────────────────────────

/// Модификатор, добавляющий отступы вокруг виджета.
///
/// Отступы добавляются как сверху, так и снизу через `ui.add_space(...)`.
pub struct Padded<W, M> {
    inner: W,
    padding: f32,
    _marker: PhantomData<M>,
}

impl<W: Widget<M>, M> Widget<M> for Padded<W, M> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        ui.add_space(self.padding);
        self.inner.render(ui, dispatch);
        ui.add_space(self.padding);
    }
}

// ─── SizedWidget ───────────────────────────────────────────────────────────────

/// Модификатор, задающий фиксированный размер для виджета.
///
/// Резервирует область указанного размера через `ui.allocate_exact_size(...)`
/// и рендерит внутренний виджет внутри неё.
pub struct SizedWidget<W, M> {
    inner: W,
    width: f32,
    height: f32,
    _marker: PhantomData<M>,
}

impl<W: Widget<M>, M> Widget<M> for SizedWidget<W, M> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        let (rect, _response) =
            ui.allocate_exact_size(egui::vec2(self.width, self.height), egui::Sense::hover());

        let mut child_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("sized_widget")
                .max_rect(rect)
                .layout(*ui.layout()),
        );
        self.inner.render(&mut child_ui, dispatch);
    }
}

// ─── Background ────────────────────────────────────────────────────────────────

/// Модификатор, задающий цвет фона для виджета.
///
/// Использует `egui::Frame::NONE.fill(color).show(...)` для отрисовки фона.
pub struct Background<W, M> {
    inner: W,
    color: egui::Color32,
    _marker: PhantomData<M>,
}

impl<W: Widget<M>, M> Widget<M> for Background<W, M> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        let frame = egui::Frame::NONE.fill(self.color);
        frame.show(ui, |ui| {
            self.inner.render(ui, dispatch);
        });
    }
}

// ─── Aligned ───────────────────────────────────────────────────────────────────

/// Модификатор, выравнивающий виджет по горизонтали.
///
/// Использует `ui.with_layout(...)` для изменения выравнивания.
pub struct Aligned<W, M> {
    inner: W,
    align: egui::Align,
    _marker: PhantomData<M>,
}

impl<W: Widget<M>, M> Widget<M> for Aligned<W, M> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        ui.with_layout(egui::Layout::top_down(self.align), |ui| {
            self.inner.render(ui, dispatch);
        });
    }
}

// ─── Clickable ─────────────────────────────────────────────────────────────────

/// Модификатор, делающий виджет кликабельным.
///
/// При клике по области виджета диспатчит заданное сообщение.
pub struct Clickable<W, M> {
    inner: W,
    msg: M,
    _marker: PhantomData<M>,
}

impl<W: Widget<M>, M: Clone> Widget<M> for Clickable<W, M> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        let (_rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::click());

        let mut child_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("clickable")
                .max_rect(_rect)
                .layout(*ui.layout()),
        );
        self.inner.render(&mut child_ui, dispatch);

        if response.clicked() {
            dispatch.dispatch(self.msg.clone());
        }
    }
}

// ─── ModifierExt ───────────────────────────────────────────────────────────────

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
            _marker: PhantomData,
        }
    }

    fn size(self, width: f32, height: f32) -> SizedWidget<Self, M> {
        SizedWidget {
            inner: self,
            width,
            height,
            _marker: PhantomData,
        }
    }

    fn background(self, color: egui::Color32) -> Background<Self, M> {
        Background {
            inner: self,
            color,
            _marker: PhantomData,
        }
    }

    fn align(self, align: egui::Align) -> Aligned<Self, M> {
        Aligned {
            inner: self,
            align,
            _marker: PhantomData,
        }
    }

    fn clickable(self, msg: M) -> Clickable<Self, M>
    where
        M: Clone,
    {
        Clickable {
            inner: self,
            msg,
            _marker: PhantomData,
        }
    }
}
