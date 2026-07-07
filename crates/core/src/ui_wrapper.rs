//! Wrapper over `egui::Ui` with Constraints support.
//!
//! Implements `Deref<Target = egui::Ui>` for compatibility with existing code.

use std::ops::{Deref, DerefMut};

use crate::Constraints;

/// Глобальный ключ для хранения constraints в Context.
fn cx_key() -> egui::Id {
    egui::Id::new("ui_wrapper_cx")
}

fn read_cx(ui: &egui::Ui) -> Constraints {
    ui.ctx()
        .data(|d| d.get_temp::<Constraints>(cx_key()).unwrap_or_default())
}

fn write_cx(ui: &egui::Ui, constraints: Constraints) {
    ui.ctx().data_mut(|d| d.insert_temp(cx_key(), constraints));
}

/// Wrapper over `egui::Ui` with Constraints support.
///
/// Two variants:
/// - `Borrowed` — wraps `&mut egui::Ui`
/// - `Owned` — wraps an owned `egui::Ui` (from `new_child`)
pub enum UiWrapper<'a> {
    /// Wraps a mutable reference to `egui::Ui`.
    Borrowed(&'a mut egui::Ui, Constraints),
    /// Wraps an owned `egui::Ui`.
    Owned(egui::Ui, Constraints),
}

impl<'a> UiWrapper<'a> {
    /// Create wrapper from a mutable reference with constraints.
    ///
    /// Записывает constraints в поле и в Context (переживает обёртки).
    pub fn new(ui: &'a mut egui::Ui, constraints: Constraints) -> Self {
        write_cx(ui, constraints);
        Self::Borrowed(ui, constraints)
    }

    /// Create wrapper from a mutable reference.
    ///
    /// Читает constraints из Context (если были установлены родителем).
    /// Если нет — использует unconstrained.
    pub fn new_unconstrained(ui: &'a mut egui::Ui) -> Self {
        let constraints = read_cx(ui);
        Self::Borrowed(ui, constraints)
    }

    /// Get current constraints.
    pub fn constraints(&self) -> &Constraints {
        match self {
            Self::Borrowed(_, c) => c,
            Self::Owned(_, c) => c,
        }
    }

    /// Get mutable reference to constraints.
    pub fn constraints_mut(&mut self) -> &mut Constraints {
        match self {
            Self::Borrowed(_, c) => c,
            Self::Owned(_, c) => c,
        }
    }

    /// Set constraints (updates field and Context).
    pub fn set_constraints(&mut self, constraints: Constraints) {
        match self {
            Self::Borrowed(ui, field) => {
                *field = constraints;
                write_cx(ui, *field);
            }
            Self::Owned(ui, field) => {
                *field = constraints;
                write_cx(ui, *field);
            }
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
        UiWrapper::Owned(child_ui, constraints)
    }

    /// Create child UiWrapper with new constraints.
    pub fn new_child_with_constraints(
        ui: &mut egui::Ui,
        builder: egui::UiBuilder,
        constraints: Constraints,
    ) -> UiWrapper<'_> {
        write_cx(ui, constraints);
        let child_ui = ui.new_child(builder);
        UiWrapper::Owned(child_ui, constraints)
    }

    fn ui_mut(&mut self) -> &mut egui::Ui {
        match self {
            Self::Borrowed(ui, _) => ui,
            Self::Owned(ui, _) => ui,
        }
    }

    fn ui_ref(&self) -> &egui::Ui {
        match self {
            Self::Borrowed(ui, _) => ui,
            Self::Owned(ui, _) => ui,
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
            let mut wrapper = UiWrapper::new(ui, constraints);
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
