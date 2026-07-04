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

/// Тип closure для `ClickableWith` — получает `Response`, `Ui` и `Dispatcher`.
pub type ClickableCallback<M> = Box<dyn Fn(&egui::Response, &egui::Ui, &Dispatcher<M>)>;

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
///
/// ИСПРАВЛЕНИЕ БАГА: кликабельная область определяется
/// реальным размером отрисованного контента, а не `available_size()`.
pub struct Clickable<W, M> {
    inner: W,
    msg: M,
    _marker: PhantomData<M>,
}

impl<W: Widget<M>, M: Clone> Widget<M> for Clickable<W, M> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        // 1. Рендерим внутренний виджет, измеряем реальный размер
        let (content_rect, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
        let mut child_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("clickable_inner")
                .max_rect(content_rect)
                .layout(*ui.layout()),
        );
        self.inner.render(&mut child_ui, dispatch);
        let widget_min = child_ui.min_size();

        // 2. Аллоцируем кликабельную область ровно по размеру контента
        let (_rect, response) = ui.allocate_exact_size(widget_min, egui::Sense::click());

        if response.clicked() {
            dispatch.dispatch(self.msg.clone());
        }
    }
}

// ─── ClickableWith ─────────────────────────────────────────────────────────────

/// Модификатор, делающий виджет кликабельным через closure.
/// При клике по области виджета диспатчит заданное сообщение через closure.
///
/// В отличие от [`Clickable`], принимает closure с доступом к `Response`,
/// `Ui` и `Dispatcher`. Позволяет выполнять произвольные UI-действия
/// при клике (например, модифицировать remember).
///
/// ИСПРАВЛЕНИЕ БАГА: кликабельная область определяется
/// реальным размером отрисованного контента, а не `available_size()`.
///
/// # Пример
///
/// ```ignore
/// let count = remember(ui, "counter", || 0i32);
/// Text::new("Кликни")
///     .clickable_with(move |response, _ui, _dispatch| {
///         if response.clicked() {
///             count.modify(|c| *c += 1);
///         }
///     })
///     .render(ui, dispatch);
/// ```
pub struct ClickableWith<W, M> {
    inner: W,
    callback: ClickableCallback<M>,
}

impl<W: Widget<M>, M: 'static> Widget<M> for ClickableWith<W, M> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        // 1. Рендерим внутренний виджет, измеряем реальный размер
        let (inner_rect, _inner_response) =
            ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
        let mut child_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("clickable_with_inner")
                .max_rect(inner_rect)
                .layout(*ui.layout()),
        );
        self.inner.render(&mut child_ui, dispatch);
        let widget_size = child_ui.min_size();

        // 2. Аллоцируем кликабельную область ровно по размеру контента
        let (_rect, response) = ui.allocate_exact_size(widget_size, egui::Sense::click());

        (self.callback)(&response, ui, dispatch);
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

    /// Сделать виджет кликабельным через closure.
    ///
    /// Closure получает `&Response`, `&Ui` и `&Dispatcher<M>`.
    /// Используется для локальных UI-действий, например изменения `remember`.
    fn clickable_with<F>(self, callback: F) -> ClickableWith<Self, M>
    where
        F: Fn(&egui::Response, &egui::Ui, &Dispatcher<M>) + 'static,
        M: 'static;
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

    fn clickable_with<F>(self, callback: F) -> ClickableWith<Self, M>
    where
        F: Fn(&egui::Response, &egui::Ui, &Dispatcher<M>) + 'static,
        M: 'static,
    {
        ClickableWith {
            inner: self,
            callback: Box::new(callback),
        }
    }
}
