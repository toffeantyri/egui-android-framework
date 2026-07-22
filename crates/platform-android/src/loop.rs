//! Состояние и итерация главного цикла.
//!
//! Вынесен из `run.rs` для разделения ответственности.
//!
//! Содержит:
//! - [`RunState`] — всё состояние главного цикла (input, graphics, rt_ctx, тайминг)
//! - [`RunState::tick()`] — одна итерация: poll → process → render

#![cfg(target_os = "android")]

use std::time::{Duration, Instant};

use crate::backend::AndroidBackend;
use crate::event::BackendEvent;
use crate::graphics::GraphicsPipeline;
use crate::input::InputState;
use egui_android_platform::Waker;
use egui_android_runtime::{Application, RuntimeContext};

/// Состояние главного цикла.
///
/// Владеет всем состоянием, необходимым для одной итерации цикла.
/// Создаётся в `run_with_backend()` и живёт всё время жизни приложения.
pub struct RunState {
    pub input_state: InputState,
    pub graphics: Option<GraphicsPipeline>,
    pub last_frame: Instant,
    pub rt_ctx: Option<RuntimeContext>,
    pub rt_ctx_initialized: bool,
    pub destroy_requested: bool,
}

impl RunState {
    /// Создать новое состояние цикла.
    pub fn new() -> Self {
        Self {
            input_state: InputState::new(),
            graphics: None,
            last_frame: Instant::now(),
            rt_ctx: None,
            rt_ctx_initialized: false,
            destroy_requested: false,
        }
    }

    /// Одна итерация главного цикла.
    ///
    /// Последовательность:
    /// 1. `backend.poll_events()` — получить события от платформы
    /// 2. Lifecycle → `lifecycle::handle_lifecycle_event()`
    /// 3. Input/Text/Insets/Dpi → `input_processing::process_backend_input()`
    /// 4. Если `destroy_requested` → destroy + return false
    /// 5. Если нет graphics → `GraphicsPipeline::try_new()`
    /// 6. Если `back_pressed` → `input_processing::process_back_pressed()`
    /// 7. `rt_ctx.check()` — проверить сигнал от data layer
    /// 8. Если пора (>= target_dt) → рендеринг:
    ///    - insets → raw_input → `app_instance.frame()`
    ///    - `GraphicsPipeline::render_frame()`
    ///
    /// Возвращает `true` — продолжать цикл, `false` — выйти.
    pub fn tick<A: Application>(
        &mut self,
        backend: &mut dyn AndroidBackend,
        app_instance: &mut A,
        egui_ctx: &egui::Context,
        waker: &Waker,
        target_dt: Duration,
    ) -> bool {
        // --- Шаг 1: poll events ---
        let backend_events = backend.poll_events();

        // --- Шаги 2-3: обработка событий ---
        for event in backend_events {
            match event {
                BackendEvent::Lifecycle(ev) => {
                    crate::lifecycle::handle_lifecycle_event(
                        ev,
                        backend,
                        app_instance,
                        egui_ctx,
                        &mut self.graphics,
                        &mut self.destroy_requested,
                    );
                }
                other => {
                    crate::input_processing::process_backend_input(
                        other,
                        &mut self.input_state,
                        egui_ctx,
                    );
                }
            }
        }

        // --- Шаг 4: проверка завершения ---
        if self.destroy_requested {
            app_instance.on_destroy();
            if let Some(ref mut g) = self.graphics {
                g.destroy();
            }
            return false;
        }

        // --- Шаг 5: инициализация GraphicsPipeline ---
        if self.graphics.is_none() {
            self.graphics = GraphicsPipeline::try_new(
                backend,
                app_instance,
                egui_ctx,
                waker,
                &mut self.rt_ctx,
                &mut self.rt_ctx_initialized,
            );

            if self.graphics.is_none() {
                // Нет EGL — ждём InitWindow
                return true;
            }
        }

        // --- Шаг 6: Back ---
        if self.input_state.back_pressed {
            crate::input_processing::process_back_pressed(
                app_instance,
                backend,
                &mut self.input_state,
            );
        }

        // --- Шаг 7: проверка уведомлений от data layer ---
        if let Some(ref mut ctx) = self.rt_ctx {
            ctx.check();
        }

        // --- Шаг 8: рендеринг ---
        let now = Instant::now();
        if now.duration_since(self.last_frame) >= target_dt {
            self.last_frame = now;

            let (w, h) = backend.window_size();
            if w == 0 || h == 0 {
                return true;
            }

            let pp = egui_ctx.pixels_per_point();

            // Получаем insets для этого кадра
            let insets = get_current_insets(backend, pp, w, h);

            let events_for_frame = std::mem::take(&mut self.input_state.events);

            let screen_rect = egui::Rect::from_min_size(
                egui::Pos2::new(insets.left, insets.top),
                egui::vec2(
                    (w as f32 / pp) - insets.left - insets.right,
                    (h as f32 / pp) - insets.top - insets.bottom,
                ),
            );

            let predicted_dt = target_dt.as_secs_f64() as f32;
            let time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64();

            let raw_input = egui::RawInput {
                screen_rect: Some(screen_rect),
                events: events_for_frame,
                predicted_dt,
                time: Some(time),
                ..egui::RawInput::default()
            };

            let full_output = app_instance.frame(egui_ctx, raw_input);

            if app_instance.request_destroy() {
                self.destroy_requested = true;
                return true;
            }

            // Синхронизируем clear color с темой Application
            // После frame() egui-стиль уже содержит panel_fill, установленный
            // Application через MaterialTheme::apply().
            {
                let style = egui_ctx.style_of(egui::Theme::Light).as_ref().clone();
                let bg = style.visuals.panel_fill;
                backend.platform_state().set_clear_color_from(bg);
            }

            // Рендеринг через GraphicsPipeline
            if let Some(ref mut g) = self.graphics {
                let clear_color = backend.platform_state().current_clear_color();
                let success = g.render_frame(
                    egui_ctx,
                    &full_output,
                    (w, h),
                    clear_color,
                    (insets.left, insets.top, insets.right, insets.bottom),
                    pp,
                    backend,
                );
                if !success {
                    // swap_buffers не удался — пересоздадим pipeline
                    let mut p = None;
                    std::mem::swap(&mut p, &mut self.graphics);
                    if let Some(mut old) = p {
                        old.destroy();
                    }
                }
            }
        }

        true
    }
}

impl Default for RunState {
    fn default() -> Self {
        Self::new()
    }
}

/// Получить текущие insets для кадра.
///
/// Приоритет:
/// 1. JNI (WindowInsets.Type.systemBars) — через PlatformState в backend
/// 2. Fallback: content_rect из backend с clamp
fn get_current_insets(
    backend: &dyn AndroidBackend,
    pp: f32,
    w: u32,
    h: u32,
) -> crate::event::Insets {
    // 1. Пробуем JNI insets через PlatformState
    let state = backend.platform_state();
    if state.insets_are_valid() {
        let px = state.insets_px();
        return crate::event::Insets {
            left: px.left as f32 / pp,
            top: px.top as f32 / pp,
            right: px.right as f32 / pp,
            bottom: px.bottom as f32 / pp,
        };
    }

    // 2. Fallback: content_rect из backend
    let (left_px, top_px, right_px, bottom_px) = backend.content_rect();
    let top_pt = (top_px as f32 / pp).clamp(0.0, 32.0);
    let bottom_pt = ((h as i32 - bottom_px) as f32 / pp).clamp(0.0, 48.0);
    let left_pt = (left_px as f32 / pp).clamp(0.0, 32.0);
    let right_pt = ((w as i32 - right_px) as f32 / pp).clamp(0.0, 32.0);

    log::info!(
        "Insets fallback: content_rect=({},{},{},{}) px -> left={:.1} top={:.1} right={:.1} bottom={:.1} pt (pp={:.2})",
        left_px, top_px, right_px, bottom_px,
        left_pt, top_pt, bottom_pt, right_pt, pp
    );

    crate::event::Insets {
        left: left_pt,
        top: top_pt,
        right: right_pt,
        bottom: bottom_pt,
    }
}
