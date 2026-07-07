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
//! # Приоритет модификаторов
//!
//! Модификаторы применяются **в порядке добавления** (первый — самый внешний):
//!
//! ```ignore
//! Modifier::new()
//!     .fill_max_width()       // 1. Резервирует всю ширину
//!     .padding(16.0)           // 2. Внутри — отступ
//!     .background(RED)         // 3. Фон рисуется поверх padding
//!     .clickable(Msg::Click)   // 4. Кликабельная область = размер контента
//! ```
//!
//! ### Порядок имеет значение
//!
//! - `.fill_max_width().padding(16.0)` — padding внутри полной ширины
//! - `.padding(16.0).fill_max_width()` — сначала padding, потом растяжение (может не дать эффекта)
//! - `.background(RED).padding(16.0)` — padding снаружи фона
//! - `.padding(16.0).background(RED)` — фон поверх padding
//!
//! ### Обратная совместимость
//!
//! Старая система (`ModifierExt` blanket-impl) продолжает работать.
//! Новая система доступна через [`ModifierApply`].
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

use egui_android_core::{widget::Widget, Dispatcher, UiWrapper};

pub use legacy::*;

pub use crate::animation::SlideDirection;
pub use value::Modifier;

mod value {
    use crate::animation::SlideDirection;
    use crate::UiWrapper;
    use egui::{Color32, CornerRadius, Response, Sense, Ui};
    use egui_android_core::Constraints;
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
        // Animation
        Fade(f32),
        Slide(SlideDirection, f32),
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
        WrapContentWidth,
        WrapContentSize,
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
        Clip(CornerRadius),
        Shadow(f32),

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
                ModifierNode::WrapContentWidth => f.debug_tuple("WrapContentWidth").finish(),
                ModifierNode::WrapContentSize => f.debug_tuple("WrapContentSize").finish(),
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
                ModifierNode::Clip(r) => f.debug_tuple("Clip").field(r).finish(),
                ModifierNode::Shadow(v) => f.debug_tuple("Shadow").field(v).finish(),
                ModifierNode::Clickable(m) => f.debug_tuple("Clickable").field(m).finish(),
                ModifierNode::ClickableWith(_) => f.debug_tuple("ClickableWith").finish(),
                ModifierNode::Fade(opacity) => f.debug_tuple("Fade").field(opacity).finish(),
                ModifierNode::Slide(direction, offset) => f
                    .debug_tuple("Slide")
                    .field(direction)
                    .field(offset)
                    .finish(),
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

        /// Ширина равна ширине содержимого (не растягивается на всю ширину родителя).
        pub fn wrap_content_width(mut self) -> Self {
            self.nodes.push(ModifierNode::WrapContentWidth);
            self
        }

        /// Размер равен размеру содержимого (и ширина, и высота — по контенту).
        pub fn wrap_content_size(mut self) -> Self {
            self.nodes.push(ModifierNode::WrapContentSize);
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
        pub fn rounding(mut self, radius: f32) -> Self {
            self.nodes.push(ModifierNode::CornerRadius(radius));
            self
        }

        /// Прозрачность (0.0 — полностью прозрачный, 1.0 — непрозрачный).
        pub fn alpha(mut self, alpha: f32) -> Self {
            self.nodes.push(ModifierNode::Alpha(alpha.clamp(0.0, 1.0)));
            self
        }

        /// Обрезка содержимого по скруглению.
        ///
        /// Полезно для контейнеров с закруглёнными углами.
        pub fn clip(mut self, rounding: CornerRadius) -> Self {
            self.nodes.push(ModifierNode::Clip(rounding));
            self
        }

        /// Тень (имитация через stroke).
        ///
        /// `elevation` — высота тени. Чем больше значение, тем толще и темнее тень.
        /// В egui нет нативной поддержки теней — реализовано через `Frame.stroke()`.
        pub fn shadow(mut self, elevation: f32) -> Self {
            self.nodes.push(ModifierNode::Shadow(elevation));
            self
        }

        // ─── Animation ──────────────────────────────────────────────────────

        /// Применить прозрачность (Fade модификатор).
        ///
        /// `opacity` от 0.0 (полностью прозрачный) до 1.0 (непрозрачный).
        pub fn fade(mut self, opacity: f32) -> Self {
            self.nodes.push(ModifierNode::Fade(opacity.clamp(0.0, 1.0)));
            self
        }

        /// Сместить контент (Slide модификатор).
        ///
        /// `direction` — направление смещения, `offset` — величина смещения в пикселях.
        pub fn slide(mut self, direction: SlideDirection, offset: f32) -> Self {
            self.nodes.push(ModifierNode::Slide(direction, offset));
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
            ui: &mut UiWrapper,
            dispatch: &Dispatcher<M>,
            content: impl FnOnce(&mut UiWrapper, &Dispatcher<M>),
        ) {
            self.apply_recursive(ui, dispatch, 0, content);
        }

        fn apply_recursive(
            &self,
            ui: &mut UiWrapper,
            dispatch: &Dispatcher<M>,
            index: usize,
            content: impl FnOnce(&mut UiWrapper, &Dispatcher<M>),
        ) {
            if index >= self.nodes.len() {
                content(ui, dispatch);
                return;
            }

            let rest = |ui: &mut UiWrapper, dispatch: &Dispatcher<M>| {
                self.apply_recursive(ui, dispatch, index + 1, content);
            };

            match &self.nodes[index] {
                // ===== PADDING =====
                ModifierNode::PaddingAll(all) => {
                    let inset = egui::Margin::symmetric(*all as i8, *all as i8);
                    egui::Frame::NONE.inner_margin(inset).show(ui, |show_ui| {
                        let mut w = UiWrapper::new_unconstrained(show_ui);
                        rest(&mut w, dispatch);
                    });
                }
                ModifierNode::PaddingHV {
                    horizontal,
                    vertical,
                } => {
                    let inset = egui::Margin::symmetric(*horizontal as i8, *vertical as i8);
                    egui::Frame::NONE.inner_margin(inset).show(ui, |show_ui| {
                        let mut w = UiWrapper::new_unconstrained(show_ui);
                        rest(&mut w, dispatch);
                    });
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
                    egui::Frame::NONE.inner_margin(inset).show(ui, |show_ui| {
                        let mut w = UiWrapper::new_unconstrained(show_ui);
                        rest(&mut w, dispatch);
                    });
                }

                // ===== SIZE CONSTRAINTS =====
                ModifierNode::FillMaxWidth => {
                    let available = ui.available_size();
                    // Создаём child_ui на всю доступную ширину (без alloc в родителе).
                    let id_salt = ui.next_auto_id();
                    let inner_rect = ui.available_rect_before_wrap();
                    let layout = *ui.layout();
                    let mut child_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .id_salt(id_salt)
                            .max_rect(egui::Rect::from_min_size(
                                inner_rect.min,
                                egui::vec2(available.x, f32::INFINITY),
                            ))
                            .layout(layout),
                    );
                    // Устанавливаем constraints.min_width = available.x, чтобы
                    // дочерние виджеты растянулись на всю ширину (match_parent).
                    // Виджеты используют ui.allocate_space_with_sense() который
                    // clamp'ит desired_size к constraints.min_width.
                    let child_constraints = Constraints::ranged(
                        available.x, // min_width = available.x
                        available.x, // max_width = available.x
                        0.0,
                        f32::INFINITY,
                    );
                    rest(
                        &mut UiWrapper::new(&mut child_ui, child_constraints),
                        dispatch,
                    );
                    let content_height = child_ui.min_size().y.max(1.0);
                    // Аллоцируем в родителе (available.x, content_height).
                    ui.allocate_exact_size(egui::vec2(available.x, content_height), Sense::hover());
                }
                ModifierNode::FillMaxSize => {
                    let available = ui.available_size();
                    let id_salt = ui.next_auto_id();
                    let child_rect = ui.allocate_exact_size(available, Sense::hover());
                    let layout = *ui.layout();
                    let mut child_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .id_salt(id_salt)
                            .max_rect(child_rect.0)
                            .layout(layout),
                    );
                    // Устанавливаем constraints = available по обеим осям.
                    let child_constraints = Constraints::exact(available.x, available.y);
                    rest(
                        &mut UiWrapper::new(&mut child_ui, child_constraints),
                        dispatch,
                    );
                }
                ModifierNode::WrapContentWidth => {
                    // Измеряем размер содержимого, рендерим один раз
                    let response = ui.scope(|scope_ui| {
                        let mut w = UiWrapper::new_unconstrained(scope_ui);
                        rest(&mut w, dispatch);
                        w.min_rect().size()
                    });
                    let content_size = response.inner;
                    // Аллоцируем ровно эту ширину
                    ui.allocate_exact_size(
                        egui::vec2(content_size.x, content_size.y),
                        Sense::hover(),
                    );
                }
                ModifierNode::WrapContentSize => {
                    // Измеряем размер содержимого, рендерим один раз
                    let response = ui.scope(|scope_ui| {
                        let mut w = UiWrapper::new_unconstrained(scope_ui);
                        rest(&mut w, dispatch);
                        w.min_rect().size()
                    });
                    let content_size = response.inner;
                    // Аллоцируем ровно этот размер
                    ui.allocate_exact_size(content_size, Sense::hover());
                }
                ModifierNode::Width(w) => {
                    let size = egui::vec2(*w, ui.available_height());
                    let id_salt = ui.next_auto_id();
                    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                    let layout = *ui.layout();
                    let mut child_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .id_salt(id_salt)
                            .max_rect(rect)
                            .layout(layout),
                    );
                    let child_constraints = Constraints::ranged(*w, *w, 0.0, f32::INFINITY);
                    rest(
                        &mut UiWrapper::new(&mut child_ui, child_constraints),
                        dispatch,
                    );
                }
                ModifierNode::Height(h) => {
                    let size = egui::vec2(ui.available_width(), *h);
                    let id_salt = ui.next_auto_id();
                    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                    let layout = *ui.layout();
                    let mut child_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .id_salt(id_salt)
                            .max_rect(rect)
                            .layout(layout),
                    );
                    let child_constraints = Constraints::ranged(0.0, f32::INFINITY, *h, *h);
                    rest(
                        &mut UiWrapper::new(&mut child_ui, child_constraints),
                        dispatch,
                    );
                }
                ModifierNode::WidthIn { min, max } => {
                    let w = ui.available_width().clamp(*min, *max);
                    let size = egui::vec2(w, ui.available_height());
                    let id_salt = ui.next_auto_id();
                    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                    let layout = *ui.layout();
                    let mut child_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .id_salt(id_salt)
                            .max_rect(rect)
                            .layout(layout),
                    );
                    rest(&mut UiWrapper::new_unconstrained(&mut child_ui), dispatch);
                }
                ModifierNode::HeightIn { min, max } => {
                    let h = ui.available_height().clamp(*min, *max);
                    let size = egui::vec2(ui.available_width(), h);
                    let id_salt = ui.next_auto_id();
                    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                    let layout = *ui.layout();
                    let mut child_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .id_salt(id_salt)
                            .max_rect(rect)
                            .layout(layout),
                    );
                    rest(&mut UiWrapper::new_unconstrained(&mut child_ui), dispatch);
                }

                // ===== APPEARANCE =====
                ModifierNode::Background(color) => {
                    egui::Frame::NONE.fill(*color).show(ui, |show_ui| {
                        let mut w = UiWrapper::new_unconstrained(show_ui);
                        rest(&mut w, dispatch);
                    });
                }
                ModifierNode::Border { width, color } => {
                    egui::Frame::NONE
                        .stroke(egui::Stroke::new(*width, *color))
                        .show(ui, |show_ui| {
                            let mut w = UiWrapper::new_unconstrained(show_ui);
                            rest(&mut w, dispatch);
                        });
                }
                ModifierNode::CornerRadius(radius) => {
                    egui::Frame::NONE
                        .corner_radius(egui::CornerRadius::same(*radius as u8))
                        .show(ui, |show_ui| {
                            let mut w = UiWrapper::new_unconstrained(show_ui);
                            rest(&mut w, dispatch);
                        });
                }
                ModifierNode::Alpha(alpha) => {
                    ui.scope(|scope_ui| {
                        scope_ui.multiply_opacity(*alpha);
                        let mut w = UiWrapper::new_unconstrained(scope_ui);
                        rest(&mut w, dispatch);
                    });
                }
                ModifierNode::Clip(rounding) => {
                    // Обрезаем содержимое по скруглению через Frame
                    let frame = egui::Frame::NONE
                        .corner_radius(*rounding)
                        .inner_margin(egui::Margin::ZERO);
                    frame.show(ui, |show_ui| {
                        let mut w = UiWrapper::new_unconstrained(show_ui);
                        rest(&mut w, dispatch);
                    });
                }
                ModifierNode::Shadow(elevation) => {
                    if *elevation > 0.0 {
                        // Имитация тени через stroke
                        let alpha = ((*elevation * 20.0) as u8).min(100);
                        let shadow_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, alpha);
                        let shadow_radius = (*elevation as u8).min(16);
                        let frame = egui::Frame::NONE
                            .stroke(egui::Stroke::new(1.0, shadow_color))
                            .corner_radius(egui::CornerRadius::same(shadow_radius))
                            .inner_margin(egui::Margin::same(*elevation as i8));
                        frame.show(ui, |show_ui| {
                            let mut w = UiWrapper::new_unconstrained(show_ui);
                            rest(&mut w, dispatch);
                        });
                    } else {
                        rest(ui, dispatch);
                    }
                }

                // ===== INTERACTION =====
                //
                // Алгоритм: рендерим контент один раз в child_ui,
                // затем alloc'им clickable-область поверх по размеру контента.
                // Это не двойной рендер — контент уже отрисован в child_ui,
                // а allocate_exact_size добавляет только Sense::click().
                ModifierNode::Clickable(msg) => {
                    let inner_rect = ui.available_rect_before_wrap();
                    let layout = *ui.layout();
                    let mut child_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .id_salt("clickable")
                            .max_rect(inner_rect)
                            .layout(layout),
                    );
                    child_ui.visuals_mut().selection.stroke = egui::Stroke::NONE;
                    rest(&mut UiWrapper::new_unconstrained(&mut child_ui), dispatch);
                    let content_size = child_ui.min_size();

                    // Аллоцируем clickable-область поверх контента
                    let (_rect, response) = ui.allocate_exact_size(content_size, Sense::click());

                    if response.clicked() {
                        dispatch.dispatch(msg.clone());
                    }
                }
                ModifierNode::ClickableWith(handler) => {
                    let inner_rect = ui.available_rect_before_wrap();
                    let layout = *ui.layout();
                    let mut child_ui = ui.new_child(
                        egui::UiBuilder::new()
                            .id_salt("clickable_with")
                            .max_rect(inner_rect)
                            .layout(layout),
                    );
                    child_ui.visuals_mut().selection.stroke = egui::Stroke::NONE;
                    rest(&mut UiWrapper::new_unconstrained(&mut child_ui), dispatch);
                    let content_size = child_ui.min_size();

                    let (_rect, response) = ui.allocate_exact_size(content_size, Sense::click());

                    if response.clicked() {
                        handler(&response, ui, dispatch);
                    }
                }

                // ===== ANIMATION =====
                ModifierNode::Fade(opacity) => {
                    ui.scope(|scope_ui| {
                        scope_ui.multiply_opacity(*opacity);
                        let mut w = UiWrapper::new_unconstrained(scope_ui);
                        rest(&mut w, dispatch);
                    });
                }
                ModifierNode::Slide(direction, offset) => {
                    let offset_vec = match direction {
                        SlideDirection::Left => egui::vec2(-*offset, 0.0),
                        SlideDirection::Right => egui::vec2(*offset, 0.0),
                        SlideDirection::Up => egui::vec2(0.0, -*offset),
                        SlideDirection::Down => egui::vec2(0.0, *offset),
                    };
                    ui.scope(|scope_ui| {
                        let max_rect = scope_ui.max_rect().translate(offset_vec);
                        let mut child_ui = scope_ui.new_child(
                            egui::UiBuilder::new()
                                .id_salt("slide")
                                .max_rect(max_rect)
                                .layout(*scope_ui.layout()),
                        );
                        rest(&mut UiWrapper::new_unconstrained(&mut child_ui), dispatch);
                    });
                }
            }
        }
    }
}

/// Виджет с применённым модификатором.
///
/// Создаётся через [`ModifierApply::modifier()`].
pub struct Modified<W, M> {
    inner: W,
    modifier: Modifier<M>,
}

impl<W: Widget<M>, M: 'static + Clone> Widget<M> for Modified<W, M> {
    fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<M>) {
        self.modifier.apply(ui, dispatch, |ui, dispatch| {
            self.inner.render(ui, dispatch);
        });
    }
}

/// Extension trait для использования новой Modifier системы.
///
/// Добавляет метод `.modifier()` ко всем типам, реализующим `Widget<M>`.
pub trait ModifierApply<M>: Widget<M> + Sized {
    /// Применить [`Modifier`] к виджету.
    fn modifier(self, modifier: Modifier<M>) -> Modified<Self, M>
    where
        M: 'static + Clone;
}

impl<T: Widget<M>, M> ModifierApply<M> for T {
    fn modifier(self, modifier: Modifier<M>) -> Modified<Self, M>
    where
        M: 'static + Clone,
    {
        Modified {
            inner: self,
            modifier,
        }
    }
}
