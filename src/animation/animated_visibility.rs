//! Виджет [`AnimatedVisibility`] — плавное появление/исчезновение
//! дочернего виджета через Fade + Scale.
//!
//! Использует `ctx.animate_bool_with_time()` для интерполяции
//! и `ui.multiply_opacity()` для прозрачности.
//! Изолирует эффекты через `ui.scope()`.

use crate::{dispatcher::Dispatcher, widgets::Widget};

/// Виджет с анимированной видимостью.
///
/// Плавно показывает или скрывает содержимое при изменении
/// флага `visible`. Эффекты: Fade (прозрачность).
///
/// # Пример
///
/// ```ignore
/// AnimatedVisibility::new(state.show_details, Duration::from_millis(300))
///     .child(Text::new("Детали"))
///     .render(ui, dispatch);
/// ```
pub struct AnimatedVisibility<M> {
    visible: bool,
    duration: f32,
    child: Option<Box<dyn Widget<M>>>,
}

impl<M: 'static> AnimatedVisibility<M> {
    /// Создать анимированную видимость.
    /// Создать анимированную видимость.
    ///
    /// * `visible` — показывать ли содержимое
    /// * `duration_secs` — длительность анимации в секундах (например, `0.3` для 300ms)
    pub fn new(visible: bool, duration_secs: f32) -> Self {
        Self {
            visible,
            duration: duration_secs,
            child: None,
        }
    }

    /// Установить дочерний виджет.
    ///
    /// Принимает любой тип, реализующий [`Widget<M>`].
    pub fn child(mut self, child: impl Widget<M> + 'static) -> Self {
        self.child = Some(Box::new(child));
        self
    }
}

impl<M: 'static> Widget<M> for AnimatedVisibility<M> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        let ctx = ui.ctx();
        let id = ui.id().with("animated_visibility");
        let progress = ctx.animate_bool_with_time(id, self.visible, self.duration);

        // Если прогресс 0 и невидимы — не рендерим
        if progress <= 0.0 {
            return;
        }

        if let Some(child) = &self.child {
            ui.scope(|ui| {
                // Применяем прозрачность для fade-эффекта
                ui.multiply_opacity(progress);
                child.render(ui, dispatch);
            });
        }
    }
}
