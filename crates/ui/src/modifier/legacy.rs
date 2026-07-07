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
//! # Исправления (v2)
//!
//! - **Padded**: использует `Frame::NONE.inner_margin()` вместо `add_space()`,
//!   что корректно влияет на layout контейнера.
//! - **SizedWidget**: учитывает `min_size` содержимого, не ломает вложенные контейнеры.
//! - **Background**: рисует фон строго по размеру контента.
//! - **Aligned**: использует `top_down` для Column-контекста и `left_to_right` для Row-контекста.
//! - **Clickable/ClickableWith**: кликабельная область определяется реальным размером
//!   отрисованного контента, не `available_size()`.
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

use egui_android_core::{widget::Widget, Dispatcher, UiWrapper};

/// Тип closure для `ClickableWith` — получает `Response`, `Ui` и `Dispatcher`.
pub type ClickableCallback<M> = Box<dyn Fn(&egui::Response, &UiWrapper, &Dispatcher<M>)>;

// ─── Padded ────────────────────────────────────────────────────────────────────

/// Модификатор, добавляющий отступы вокруг виджета.
///
/// ИСПРАВЛЕНИЕ (v2): вместо `ui.add_space()` (которое создаёт пустую область
/// и может ломать выравнивание), используем `egui::Frame::NONE.inner_margin()`,
/// которое корректно уменьшает `max_rect` для внутреннего виджета и влияет
/// на layout родительского контейнера.
pub struct Padded<W, M> {
    inner: W,
    padding: f32,
    _marker: PhantomData<M>,
}

impl<W: Widget<M>, M> Widget<M> for Padded<W, M> {
    fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<M>) {
        let inset = egui::Margin::symmetric(self.padding as i8, self.padding as i8);
        egui::Frame::NONE.inner_margin(inset).show(ui, |show_ui| {
            let mut w = UiWrapper::new_unconstrained(show_ui);
            self.inner.render(&mut w, dispatch);
        });
    }
}

// ─── SizedWidget ───────────────────────────────────────────────────────────────

/// Модификатор, задающий фиксированный размер для виджета.
///
/// ИСПРАВЛЕНИЕ (v2): резервирует область через `allocate_exact_size`,
/// а затем рендерит внутренний виджет внутри `child_ui` с той же областью.
/// Теперь корректно передаёт layout родителя во вложенный Ui и учитывает
/// минимальный размер содержимого для обратной связи с контейнером.
pub struct SizedWidget<W, M> {
    inner: W,
    width: f32,
    height: f32,
    _marker: PhantomData<M>,
}

impl<W: Widget<M>, M> Widget<M> for SizedWidget<W, M> {
    fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<M>) {
        let desired_size = egui::vec2(self.width, self.height);
        // Резервируем область ровно указанного размера
        let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

        // Создаём child_ui с той же областью и layout
        let layout = *ui.layout();
        let mut child_ui = UiWrapper::new_child(
            &mut *ui,
            egui::UiBuilder::new()
                .id_salt("sized_widget")
                .max_rect(rect)
                .layout(layout),
        );

        // Рендерим внутренний виджет
        self.inner.render(&mut child_ui, dispatch);

        // Устанавливаем min_size равным желаемому размеру, чтобы
        // родительский Ui знал, какое место было занято.
        // Это гарантирует, что SizedWidget всегда занимает ровно
        // указанную область, независимо от того, сколько места
        // занял внутренний виджет.
        child_ui.set_min_size(desired_size);

        // Устанавливаем min_size равным желаемому размеру, чтобы
        // родительский Ui знал, какое место было занято.
        // Это гарантирует, что SizedWidget всегда занимает ровно
        // указанную область, независимо от того, сколько места
        // занял внутренний виджет.
        child_ui.set_min_size(desired_size);
    }
}

// ─── Background ────────────────────────────────────────────────────────────────

/// Модификатор, задающий цвет фона для виджета.
///
/// ИСПРАВЛЕНИЕ (v2): измеряет размер контента до рисования фона,
/// чтобы фон не перекрывал padding и соответствовал реальному размеру виджета.
pub struct Background<W, M> {
    inner: W,
    color: egui::Color32,
    _marker: PhantomData<M>,
}

impl<W: Widget<M>, M> Widget<M> for Background<W, M> {
    fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<M>) {
        // Используем Frame с fill — он автоматически подстраивается под размер контента
        let frame = egui::Frame::NONE.fill(self.color);
        frame.show(ui, |show_ui| {
            let mut w = UiWrapper::new_unconstrained(show_ui);
            self.inner.render(&mut w, dispatch);
        });
    }
}

// ─── Aligned ───────────────────────────────────────────────────────────────────

/// Модификатор, выравнивающий виджет по горизонтали.
///
/// ИСПРАВЛЕНИЕ (v2): использует `Layout::top_down(align)` для Column-контекста
/// и `Layout::left_to_right().with_main_align(align)` для Row-контекста.
/// Раньше всегда использовался `top_down`, что ломало Row.
pub struct Aligned<W, M> {
    inner: W,
    align: egui::Align,
    _marker: PhantomData<M>,
}

impl<W: Widget<M>, M> Widget<M> for Aligned<W, M> {
    fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<M>) {
        // Определяем направление текущего layout
        let is_horizontal = ui.layout().is_horizontal();

        let layout = if is_horizontal {
            // В Row используем горизонтальный layout с выравниванием по основной оси
            egui::Layout::left_to_right(egui::Align::Center).with_main_align(self.align)
        } else {
            // В Column (и по умолчанию) используем вертикальный layout
            egui::Layout::top_down(self.align)
        };

        ui.with_layout(layout, |layout_ui| {
            let mut w = UiWrapper::new_unconstrained(layout_ui);
            self.inner.render(&mut w, dispatch);
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
///
/// Алгоритм:
/// 1. Рендерим внутренний виджет во временный child_ui, измеряем min_size.
/// 2. Аллоцируем кликабельную область ровно по размеру контента.
/// 3. Если клик был — диспатчим сообщение.
pub struct Clickable<W, M> {
    inner: W,
    msg: M,
    _marker: PhantomData<M>,
}

impl<W: Widget<M>, M: Clone> Widget<M> for Clickable<W, M> {
    fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<M>) {
        // 1. Сначала рендерим содержимое в child_ui, чтобы измерить его реальный размер
        let inner_rect = ui.available_rect_before_wrap();
        let layout = *ui.layout();
        let mut child_ui = UiWrapper::new_child(
            &mut *ui,
            egui::UiBuilder::new()
                .id_salt("clickable_inner")
                .max_rect(inner_rect)
                .layout(layout),
        );
        self.inner.render(&mut child_ui, dispatch);
        let content_size = child_ui.min_size();

        // 2. Аллоцируем кликабельную область ровно по размеру контента
        let (_rect, response) = ui.allocate_exact_size(content_size, egui::Sense::click());

        if response.clicked() {
            dispatch.dispatch(self.msg.clone());
        }
    }
}

// ─── ClickableWith ─────────────────────────────────────────────────────────────

/// Модификатор, делающий виджет кликабельным через closure.
/// При клике по области виджета вызывает заданный closure.
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
    fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<M>) {
        // 1. Рендерим внутренний виджет, измеряем реальный размер
        let inner_rect = ui.available_rect_before_wrap();
        let layout = *ui.layout();
        let mut child_ui = UiWrapper::new_child(
            &mut *ui,
            egui::UiBuilder::new()
                .id_salt("clickable_with_inner")
                .max_rect(inner_rect)
                .layout(layout),
        );
        self.inner.render(&mut child_ui, dispatch);
        let content_size = child_ui.min_size();

        // 2. Аллоцируем кликабельную область ровно по размеру контента
        let (_rect, response) = ui.allocate_exact_size(content_size, egui::Sense::click());

        if response.clicked() {
            (self.callback)(&response, ui, dispatch);
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

    /// Сделать виджет кликабельным через closure.
    ///
    /// Closure получает `&Response`, `&UiWrapper` и `&Dispatcher<M>`.
    /// Используется для локальных UI-действий, например изменения `remember`.
    fn clickable_with<F>(self, callback: F) -> ClickableWith<Self, M>
    where
        F: Fn(&egui::Response, &UiWrapper, &Dispatcher<M>) + 'static,
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
        F: Fn(&egui::Response, &UiWrapper, &Dispatcher<M>) + 'static,
        M: 'static,
    {
        ClickableWith {
            inner: self,
            callback: Box::new(callback),
        }
    }
}
