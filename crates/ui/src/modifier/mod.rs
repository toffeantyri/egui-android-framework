//! Система модификаторов — единый value type `Modifier<M>` с цепочкой методов.
//!
//! Новая система модификаторов (Compose-like), где все модификаторы собраны
//! в один тип `Modifier<M>` с методами-цепочками.
//!
//! # Отличия от старой системы
//!
//! Старая система (`legacy`) использует wrapper-структуры (`Padded<W,M>`,
//! `Background<W,M>`, `Clickable<W,M>`), каждая из которых реализует `Widget<M>`.
//! Порядок модификаторов задаётся через вложенность типов.
//!
//! Новая система использует единый `Modifier<M>` с вектором узлов,
//! которые применяются рекурсивно. Это позволяет:
//! - передавать модификаторы как value
//! - создавать цепочки методов: `.padding(8.0).background(RED).clickable(msg)`
//! - комбинировать layout, appearance и interaction модификаторы
//!
//! # Обратная совместимость
//!
//! Старая система (`ModifierExt` blanket-impl) продолжает работать.
//! Новая система доступна через [`ModifierExt2`].
//!
//! ```ignore
//! // Новый API
//! Text::new("Hello")
//!     .modifier(Modifier::new().padding(16.0).background(RED))
//!     .render(ui, dispatch);
//!
//! // Старый API (продолжает работать)
//! Text::new("Hello")
//!     .padding(16.0)
//!     .background(RED)
//!     .render(ui, dispatch);
//! ```

pub mod legacy;

use egui_android_core::{widget::Widget, Dispatcher};

pub use legacy::*;

pub use value::Modifier;

mod value {
    use egui::{Color32, Rect, Response, Sense, Ui};
    use egui_android_core::Dispatcher;

    /// Единый тип модификатора — value type с цепочкой методов.
    ///
    /// Generic `M` обеспечивает type-safe dispatch для `clickable`.
    ///
    /// # Пример
    ///
    /// ```ignore
    /// Modifier::new()
    ///     .padding(16.0)
    ///     .fill_max_width()
    ///     .background(Color32::DARK_GRAY)
    ///     .clickable(Msg::Clicked)
    /// ```
    pub struct Modifier<M = ()> {
        pub(crate) nodes: Vec<ModifierNode<M>>,
    }

    /// Внутреннее представление одного модификатора.
    pub(crate) enum ModifierNode<M> {
        // Padding
        PaddingAll(f32),
        PaddingHV {
            horizontal: f32,
            vertical: f32,
        },
        PaddingEdges {
            left: f32,
            top: f32,
            right: f32,
            bottom: f32,
        },

        // Size constraints
        FillMaxWidth,
        FillMaxSize,
        Width(f32),
        Height(f32),
        WidthIn {
            min: f32,
            max: f32,
        },
        HeightIn {
            min: f32,
            max: f32,
        },

        // Appearance
        Background(Color32),
        Border {
            width: f32,
            color: Color32,
        },
        CornerRadius(f32),
        Alpha(f32),

        // Interaction
        Clickable(M),
        ClickableWith(Box<dyn Fn(&Response, &Ui, &Dispatcher<M>)>),
    }

    // Реализуем Debug вручную из-за ClickableWith
    impl<M: std::fmt::Debug> std::fmt::Debug for ModifierNode<M> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ModifierNode::PaddingAll(v) => f.debug_tuple("PaddingAll").field(v).finish(),
                ModifierNode::PaddingHV {
                    horizontal,
                    vertical,
                } => f
                    .debug_struct("PaddingHV")
                    .field("horizontal", horizontal)
                    .field("vertical", vertical)
                    .finish(),
                ModifierNode::PaddingEdges {
                    left,
                    top,
                    right,
                    bottom,
                } => f
                    .debug_struct("PaddingEdges")
                    .field("left", left)
                    .field("top", top)
                    .field("right", right)
                    .field("bottom", bottom)
                    .finish(),
                ModifierNode::FillMaxWidth => f.debug_tuple("FillMaxWidth").finish(),
                ModifierNode::FillMaxSize => f.debug_tuple("FillMaxSize").finish(),
                ModifierNode::Width(v) => f.debug_tuple("Width").field(v).finish(),
                ModifierNode::Height(v) => f.debug_tuple("Height").field(v).finish(),
                ModifierNode::WidthIn { min, max } => f
                    .debug_struct("WidthIn")
                    .field("min", min)
                    .field("max", max)
                    .finish(),
                ModifierNode::HeightIn { min, max } => f
                    .debug_struct("HeightIn")
                    .field("min", min)
                    .field("max", max)
                    .finish(),
                ModifierNode::Background(c) => f.debug_tuple("Background").field(c).finish(),
                ModifierNode::Border { width, color } => f
                    .debug_struct("Border")
                    .field("width", width)
                    .field("color", color)
                    .finish(),
                ModifierNode::CornerRadius(v) => f.debug_tuple("CornerRadius").field(v).finish(),
                ModifierNode::Alpha(v) => f.debug_tuple("Alpha").field(v).finish(),
                ModifierNode::Clickable(m) => f.debug_tuple("Clickable").field(m).finish(),
                ModifierNode::ClickableWith(_) => f.debug_tuple("ClickableWith").finish(),
            }
        }
    }

    impl<M> Modifier<M> {
        /// Создать пустой модификатор.
        pub fn new() -> Self {
            Self { nodes: Vec::new() }
        }

        // ─── Padding ────────────────────────────────────────────────────────

        /// Одинаковый отступ со всех сторон.
        pub fn padding(mut self, all: f32) -> Self {
            self.nodes.push(ModifierNode::PaddingAll(all));
            self
        }

        /// Горизонтальный и вертикальный отступ.
        pub fn padding_hv(mut self, horizontal: f32, vertical: f32) -> Self {
            self.nodes.push(ModifierNode::PaddingHV {
                horizontal,
                vertical,
            });
            self
        }

        /// Отступ по сторонам: left, top, right, bottom.
        pub fn padding_edges(mut self, left: f32, top: f32, right: f32, bottom: f32) -> Self {
            self.nodes.push(ModifierNode::PaddingEdges {
                left,
                top,
                right,
                bottom,
            });
            self
        }

        // ─── Size ───────────────────────────────────────────────────────────

        /// Занять всю ширину родителя.
        pub fn fill_max_width(mut self) -> Self {
            self.nodes.push(ModifierNode::FillMaxWidth);
            self
        }

        /// Занять всё доступное пространство.
        pub fn fill_max_size(mut self) -> Self {
            self.nodes.push(ModifierNode::FillMaxSize);
            self
        }

        /// Фиксированная ширина.
        pub fn width(mut self, w: f32) -> Self {
            self.nodes.push(ModifierNode::Width(w));
            self
        }

        /// Фиксированная высота.
        pub fn height(mut self, h: f32) -> Self {
            self.nodes.push(ModifierNode::Height(h));
            self
        }

        /// Ограничение ширины: min..=max.
        pub fn width_in(mut self, min: f32, max: f32) -> Self {
            self.nodes.push(ModifierNode::WidthIn { min, max });
            self
        }

        /// Ограничение высоты: min..=max.
        pub fn height_in(mut self, min: f32, max: f32) -> Self {
            self.nodes.push(ModifierNode::HeightIn { min, max });
            self
        }

        // ─── Appearance ─────────────────────────────────────────────────────

        /// Цвет фона.
        pub fn background(mut self, color: Color32) -> Self {
            self.nodes.push(ModifierNode::Background(color));
            self
        }

        /// Рамка.
        pub fn border(mut self, width: f32, color: Color32) -> Self {
            self.nodes.push(ModifierNode::Border { width, color });
            self
        }

        /// Скругление углов.
        pub fn corner_radius(mut self, radius: f32) -> Self {
            self.nodes.push(ModifierNode::CornerRadius(radius));
            self
        }

        /// Прозрачность (0.0 — полностью прозрачный, 1.0 — непрозрачный).
        pub fn alpha(mut self, alpha: f32) -> Self {
            self.nodes.push(ModifierNode::Alpha(alpha.clamp(0.0, 1.0)));
            self
        }

        // ─── Interaction ────────────────────────────────────────────────────

        /// Сделать кликабельным — при клике диспатчит указанное сообщение.
        ///
        /// Кликабельная область = реальный размер контента, а не вся доступная область.
        /// Это исправляет критический баг старой реализации.
        pub fn clickable(mut self, msg: M) -> Self {
            self.nodes.push(ModifierNode::Clickable(msg));
            self
        }

        /// Сделать кликабельным через closure.
        ///
        /// Closure получает `&Response`, `&Ui` и `&Dispatcher<M>`.
        /// Кликабельная область = реальный размер контента.
        pub fn clickable_with<F>(mut self, f: F) -> Self
        where
            F: Fn(&Response, &Ui, &Dispatcher<M>) + 'static,
            M: 'static,
        {
            self.nodes.push(ModifierNode::ClickableWith(Box::new(f)));
            self
        }
    }

    impl<M> Default for Modifier<M> {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<M: 'static + Clone> Modifier<M> {
        /// Применить все модификаторы вокруг content-замыкания.
        ///
        /// Модификаторы применяются **в порядке добавления**:
        /// первый добавленный — самый внешний.
        pub fn apply(
            &self,
            ui: &mut Ui,
            dispatch: &Dispatcher<M>,
            content: impl FnOnce(&mut Ui, &Dispatcher<M>),
        ) {
            self.apply_recursive(ui, dispatch, 0, content);
        }

        fn apply_recursive(
            &self,
            ui: &mut Ui,
            dispatch: &Dispatcher<M>,
            index: usize,
            content: impl FnOnce(&mut Ui, &Dispatcher<M>),
        ) {
            if index >= self.nodes.len() {
                content(ui, dispatch);
                return;
            }

            let rest = |ui: &mut Ui, dispatch: &Dispatcher<M>| {
                self.apply_recursive(ui, dispatch, index + 1, content);
            };

            match &self.nodes[index] {
                // ===== PADDING =====
                ModifierNode::PaddingAll(all) => {
                    let inset = egui::Margin::symmetric(*all as i8, *all as i8);
                    egui::Frame::NONE
                        .inner_margin(inset)
                        .show(ui, |ui| rest(ui, dispatch));
                }
                ModifierNode::PaddingHV {
                    horizontal,
                    vertical,
                } => {
                    let inset = egui::Margin::symmetric(*horizontal as i8, *vertical as i8);
                    egui::Frame::NONE
                        .inner_margin(inset)
                        .show(ui, |ui| rest(ui, dispatch));
                }
                ModifierNode::PaddingEdges {
                    left,
                    top,
                    right,
                    bottom,
                } => {
                    let left = *left as i8;
                    let right = *right as i8;
                    let top = *top as i8;
                    let bottom = *bottom as i8;
                    let inset = egui::Margin {
                        left,
                        right,
                        top,
                        bottom,
                    };
                    egui::Frame::NONE
                        .inner_margin(inset)
                        .show(ui, |ui| rest(ui, dispatch));
                }

                // ===== SIZE CONSTRAINTS =====
                ModifierNode::FillMaxWidth => {
                    let available = ui.available_width();
                    ui.allocate_ui_with_layout(
                        egui::vec2(available, ui.available_height()),
                        *ui.layout(),
                        |ui| rest(ui, dispatch),
                    );
                }
                ModifierNode::FillMaxSize => {
                    let available = ui.available_size();
                    ui.allocate_ui_with_layout(available, *ui.layout(), |ui| {
                        rest(ui, dispatch);
                    });
                }
                ModifierNode::Width(w) => {
                    ui.allocate_ui_with_layout(
                        egui::vec2(*w, ui.available_height()),
                        *ui.layout(),
                        |ui| rest(ui, dispatch),
                    );
                }
                ModifierNode::Height(h) => {
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), *h),
                        *ui.layout(),
                        |ui| rest(ui, dispatch),
                    );
                }
                ModifierNode::WidthIn { min, max } => {
                    let w = ui.available_width().clamp(*min, *max);
                    ui.allocate_ui_with_layout(
                        egui::vec2(w, ui.available_height()),
                        *ui.layout(),
                        |ui| rest(ui, dispatch),
                    );
                }
                ModifierNode::HeightIn { min, max } => {
                    let h = ui.available_height().clamp(*min, *max);
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), h),
                        *ui.layout(),
                        |ui| rest(ui, dispatch),
                    );
                }

                // ===== APPEARANCE =====
                ModifierNode::Background(color) => {
                    egui::Frame::NONE
                        .fill(*color)
                        .show(ui, |ui| rest(ui, dispatch));
                }
                ModifierNode::Border { width, color } => {
                    egui::Frame::NONE
                        .stroke(egui::Stroke::new(*width, *color))
                        .show(ui, |ui| rest(ui, dispatch));
                }
                ModifierNode::CornerRadius(radius) => {
                    egui::Frame::NONE
                        .corner_radius(egui::CornerRadius::same(*radius as u8))
                        .show(ui, |ui| rest(ui, dispatch));
                }
                ModifierNode::Alpha(alpha) => {
                    ui.scope(|ui| {
                        ui.multiply_opacity(*alpha);
                        rest(ui, dispatch);
                    });
                }

                // ===== INTERACTION =====
                ModifierNode::Clickable(msg) => {
                    // ИСПРАВЛЕНИЕ БАГА:
                    // 1. Сначала рендерим content, чтобы измерить его размер
                    // 2. Затем создаём интерактивную область ровно по этому размеру
                    let mut content_rect = Rect::NOTHING;

                    ui.scope(|ui| {
                        rest(ui, dispatch);
                        content_rect = ui.min_rect();
                    });

                    let response = ui.interact(content_rect, ui.next_auto_id(), Sense::click());
                    if response.clicked() {
                        dispatch.dispatch(msg.clone());
                    }
                }
                ModifierNode::ClickableWith(handler) => {
                    let mut content_rect = Rect::NOTHING;

                    ui.scope(|ui| {
                        rest(ui, dispatch);
                        content_rect = ui.min_rect();
                    });

                    let response = ui.interact(content_rect, ui.next_auto_id(), Sense::click());
                    if response.clicked() {
                        handler(&response, ui, dispatch);
                    }
                }
            }
        }
    }
}

/// Виджет с применённым модификатором.
///
/// Создаётся через [`ModifierExt2::modifier()`].
pub struct ModifiedWidget<W, M> {
    inner: W,
    modifier: Modifier<M>,
}

impl<W: Widget<M>, M: 'static + Clone> Widget<M> for ModifiedWidget<W, M> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        self.modifier.apply(ui, dispatch, |ui, dispatch| {
            self.inner.render(ui, dispatch);
        });
    }
}

/// Extension trait для использования новой Modifier системы.
///
/// Добавляет метод `.modifier()` ко всем типам, реализующим `Widget<M>`.
pub trait ModifierExt2<M>: Widget<M> + Sized {
    /// Применить [`Modifier`] к виджету.
    fn modifier(self, modifier: Modifier<M>) -> ModifiedWidget<Self, M>
    where
        M: 'static + Clone;
}

impl<T: Widget<M>, M> ModifierExt2<M> for T {
    fn modifier(self, modifier: Modifier<M>) -> ModifiedWidget<Self, M>
    where
        M: 'static + Clone,
    {
        ModifiedWidget {
            inner: self,
            modifier,
        }
    }
}
