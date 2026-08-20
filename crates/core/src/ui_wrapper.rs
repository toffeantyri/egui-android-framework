//! Wrapper over `egui::Ui` with Constraints support.
//!
//! Implements `Deref<Target = egui::Ui>` for compatibility with existing code.
//!
//! Constraints хранятся в `egui::Context::data()` (единственный источник правды),
//! а не в поле UiWrapper. Это гарантирует, что constraints переживают
//! `Frame::show()` и `ScrollArea::show()`, которые создают новый `egui::Ui`.

use std::ops::{Deref, DerefMut};

use crate::Constraints;

/// Глобальный ключ для хранения constraints в Context.
fn cx_key() -> egui::Id {
    egui::Id::new("ui_wrapper_cx")
}

/// Глобальный ключ для накопленной трансляции координат (сдвиг контента внутри
/// scroll-контейнеров). Аналог constraints: хранится в Context, чтобы переживать
/// `Frame::show` / `ScrollArea::show`. См. `crates/ui/src/text_selection/coords.rs`.
fn shift_key() -> egui::Id {
    egui::Id::new("ui_wrapper_shift")
}

fn read_cx(ui: &egui::Ui) -> Constraints {
    ui.ctx()
        .data(|d| d.get_temp::<Constraints>(cx_key()).unwrap_or_default())
}

fn write_cx(ui: &egui::Ui, constraints: Constraints) {
    ui.ctx().data_mut(|d| d.insert_temp(cx_key(), constraints));
}

/// Текущая накопленная трансляция координат (сумма сдвигов скролл-контейнеров
/// по цепочке), хранимая в Context.
///
/// Хранится через `*_persisted`, а не `*_temp`: значение пишется scrollable-Column
/// в КОНЦЕ кадра (после рендера контента) и должно быть доступно виджетам в
/// СЛЕДУЮЩЕМ кадре. temp-данные очищаются в конце каждого кадра, поэтому потеряли бы сдвиг.
pub fn read_shift(ui: &egui::Ui) -> egui::Vec2 {
    ui.ctx().data_mut(|d| {
        d.get_persisted::<egui::Vec2>(shift_key())
            .unwrap_or(egui::Vec2::ZERO)
    })
}

/// Записать накопленную трансляцию координат в Context (переживает кадры).
pub fn write_shift(ui: &egui::Ui, shift: egui::Vec2) {
    ui.ctx()
        .data_mut(|d| d.insert_persisted(shift_key(), shift));
}

/// Wrapper over `egui::Ui` with Constraints support.
///
/// Two variants:
/// - `Borrowed` — wraps `&mut egui::Ui`
/// - `Owned` — wraps an owned `egui::Ui` (from `new_child`)
///
/// Constraints хранятся только в `egui::Context::data()`, не в поле.
pub enum UiWrapper<'a> {
    /// Wraps a mutable reference to `egui::Ui`.
    Borrowed(&'a mut egui::Ui),
    /// Wraps an owned `egui::Ui` (boxed to reduce enum size).
    Owned(Box<egui::Ui>),
}

impl<'a> UiWrapper<'a> {
    /// Create wrapper from a mutable reference with constraints.
    ///
    /// Записывает constraints в Context (переживает обёртки).
    pub fn new(ui: &'a mut egui::Ui, constraints: Constraints) -> Self {
        write_cx(ui, constraints);
        Self::Borrowed(ui)
    }

    /// Create wrapper from a mutable reference.
    ///
    /// Читает constraints из Context (если были установлены родителем).
    /// Если нет — использует unconstrained.
    pub fn new_unconstrained(ui: &'a mut egui::Ui) -> Self {
        Self::Borrowed(ui)
    }

    /// Get current constraints from Context.
    pub fn constraints(&self) -> Constraints {
        match self {
            Self::Borrowed(ui) => read_cx(ui),
            Self::Owned(ui) => read_cx(ui),
        }
    }

    /// Set constraints (updates Context).
    pub fn set_constraints(&mut self, constraints: Constraints) {
        match self {
            Self::Borrowed(ui) => write_cx(ui, constraints),
            Self::Owned(ui) => write_cx(ui, constraints),
        }
    }

    /// Create child UiWrapper inheriting current constraints.
    ///
    /// Статический метод — принимает `&mut egui::Ui` напрямую,
    /// чтобы избежать проблем с lifetime при возврате Owned.
    pub fn new_child(ui: &mut egui::Ui, builder: egui::UiBuilder) -> UiWrapper<'_> {
        let constraints = read_cx(ui);
        write_cx(ui, constraints);
        let child_ui = ui.new_child(builder);
        UiWrapper::Owned(Box::new(child_ui))
    }

    /// Create a child `Ui` with the given constraints.
    pub fn new_child_with_constraints(
        ui: &mut egui::Ui,
        builder: egui::UiBuilder,
        constraints: Constraints,
    ) -> UiWrapper<'_> {
        write_cx(ui, constraints);
        let child_ui = ui.new_child(builder);
        UiWrapper::Owned(Box::new(child_ui))
    }

    fn ui_mut(&mut self) -> &mut egui::Ui {
        match self {
            Self::Borrowed(ui) => ui,
            Self::Owned(ui) => ui,
        }
    }

    fn ui_ref(&self) -> &egui::Ui {
        match self {
            Self::Borrowed(ui) => ui,
            Self::Owned(ui) => ui,
        }
    }

    /// Allocate space respecting constraints.
    pub fn allocate_space(&mut self, desired_size: egui::Vec2) -> (egui::Rect, egui::Response) {
        let clamped_size = self.constraints().clamp_size(desired_size);
        self.ui_mut()
            .allocate_exact_size(clamped_size, egui::Sense::hover())
    }

    /// Allocate space with sense respecting constraints.
    pub fn allocate_space_with_sense(
        &mut self,
        desired_size: egui::Vec2,
        sense: egui::Sense,
    ) -> (egui::Rect, egui::Response) {
        let clamped_size = self.constraints().clamp_size(desired_size);
        self.ui_mut().allocate_exact_size(clamped_size, sense)
    }
}

impl<'a> Deref for UiWrapper<'a> {
    type Target = egui::Ui;

    fn deref(&self) -> &Self::Target {
        self.ui_ref()
    }
}

impl<'a> DerefMut for UiWrapper<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ui_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Sense;

    fn with_ui(f: impl FnOnce(&mut egui::Ui)) {
        let ctx = egui::Context::default();
        let f = std::cell::RefCell::new(Some(f));
        let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let f = f.borrow_mut().take().unwrap();
                f(ui);
            });
        });
    }

    #[test]
    fn test_deref_works() {
        with_ui(|ui| {
            let wrapper = UiWrapper::new_unconstrained(ui);
            let _avail = wrapper.available_width();
            let _id = wrapper.next_auto_id();
        });
    }

    #[test]
    fn test_constraints_default() {
        with_ui(|ui| {
            let wrapper = UiWrapper::new_unconstrained(ui);
            assert_eq!(wrapper.constraints().min_width, 0.0);
            assert!(wrapper.constraints().max_width.is_infinite());
        });
    }

    #[test]
    fn test_constraints_custom() {
        with_ui(|ui| {
            let constraints = Constraints::ranged(10.0, 200.0, 20.0, 300.0);
            let wrapper = UiWrapper::new(ui, constraints);
            assert_eq!(wrapper.constraints().min_width, 10.0);
        });
    }

    #[test]
    fn test_allocate_space_clamps() {
        with_ui(|ui| {
            let constraints = Constraints::exact(200.0, 100.0);
            let mut wrapper = UiWrapper::new(ui, constraints);

            let (rect, _) = wrapper.allocate_space(egui::vec2(50.0, 30.0));
            assert_eq!(rect.width(), 200.0);
            assert_eq!(rect.height(), 100.0);
        });
    }

    #[test]
    fn test_allocate_space_with_sense() {
        with_ui(|ui| {
            let mut wrapper = UiWrapper::new_unconstrained(ui);
            let (rect, response) =
                wrapper.allocate_space_with_sense(egui::vec2(100.0, 50.0), Sense::click());
            assert_eq!(rect.width(), 100.0);
            assert_eq!(rect.height(), 50.0);
            assert!(response.sense == Sense::click());
        });
    }

    #[test]
    fn test_new_child_inherits_constraints() {
        with_ui(|ui| {
            let constraints = Constraints::ranged(10.0, 200.0, 0.0, f32::INFINITY);
            let mut wrapper = UiWrapper::new(ui, constraints);

            let child = UiWrapper::new_child(&mut *wrapper, egui::UiBuilder::new());
            assert_eq!(child.constraints().min_width, 10.0);
            assert_eq!(child.constraints().max_width, 200.0);
        });
    }

    #[test]
    fn test_new_child_with_constraints() {
        with_ui(|ui| {
            let mut wrapper = UiWrapper::new_unconstrained(ui);
            let child_constraints = Constraints::exact(300.0, 150.0);

            let child = UiWrapper::new_child_with_constraints(
                &mut *wrapper,
                egui::UiBuilder::new(),
                child_constraints,
            );
            assert_eq!(child.constraints().min_width, 300.0);
            assert_eq!(child.constraints().max_width, 300.0);
        });
    }

    #[test]
    fn test_new_child_via_frame_keeps_constraints() {
        with_ui(|ui| {
            let constraints = Constraints::exact(200.0, 100.0);
            let mut wrapper = UiWrapper::new(ui, constraints);

            egui::Frame::NONE
                .inner_margin(egui::Margin::symmetric(8, 8))
                .show(&mut *wrapper, |inner_ui| {
                    let inner = UiWrapper::new_unconstrained(inner_ui);
                    assert_eq!(
                        inner.constraints().min_width,
                        200.0,
                        "constraints должны пережить Frame::show"
                    );
                });
        });
    }

    #[test]
    fn test_set_constraints() {
        with_ui(|ui| {
            let mut wrapper = UiWrapper::new_unconstrained(ui);
            assert!(wrapper.constraints().max_width.is_infinite());

            wrapper.set_constraints(Constraints::exact(100.0, 50.0));
            assert_eq!(wrapper.constraints().min_width, 100.0);
        });
    }

    #[test]
    fn test_deref_mut() {
        with_ui(|ui| {
            let mut wrapper = UiWrapper::new_unconstrained(ui);
            wrapper.set_min_width(200.0);
            assert!(wrapper.min_rect().width() >= 200.0);
        });
    }

    #[test]
    fn test_owning_child() {
        with_ui(|ui| {
            let constraints = Constraints::exact(150.0, 75.0);
            let mut parent = UiWrapper::new(ui, constraints);
            let child = UiWrapper::new_child(&mut *parent, egui::UiBuilder::new());
            assert_eq!(child.constraints().min_width, 150.0);
        });
    }
}
