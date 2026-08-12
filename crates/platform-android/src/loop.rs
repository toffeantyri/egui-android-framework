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
use crate::platform_state::PlatformState;
use egui::viewport::ViewportId;
use egui_android_platform::Waker;
use egui_android_runtime::{
    keyboard_controller_id, next_ime_field_after, Application, KeyboardController, RuntimeContext,
};

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
    repaint_delay: Duration,
    last_theme: Option<egui_android_platform::SystemTheme>,
    /// Пользователь нажал Next (IME_ACTION_NEXT): после `frame()` перевести
    /// фокус на следующее поле из реестра. `_due` — флаг «пора применять»:
    /// переключение откладывается на кадр после доставки preedit в текущее поле,
    /// иначе ввод ушёл бы в новое поле (в старом текст не появился бы).
    move_focus_next_pending: bool,
    move_focus_next_due: bool,
    /// Изолированный редьюсер ввода (команды → буферные операции). Единственный
    /// источник состояния композиции; см. [`crate::ime_service`].
    ime_service: crate::ime_service::DefaultImeService,
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
            repaint_delay: Duration::ZERO,
            last_theme: None,
            move_focus_next_pending: false,
            move_focus_next_due: false,
            ime_service: crate::ime_service::DefaultImeService::default(),
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
        platform_state: &PlatformState,
    ) -> bool {
        // --- Шаг 1: poll events (с timeout на основе repaint_delay) ---
        let timeout = crate::ime_service::poll_timeout(
            self.graphics.is_none(),
            self.repaint_delay,
            platform_state.has_ime_cmds(),
        );

        let poll_start = Instant::now();
        let backend_events = backend.poll_events(timeout);
        let poll_elapsed = poll_start.elapsed();

        // log::info!(
        //     "LOOP: poll_events(timeout={:?}) -> {} событий, спали {:?}",
        //     timeout,
        //     backend_events.len(),
        //     poll_elapsed,
        // );

        // Есть ли IME-команды, ожидающие обработки (считаем событиями для этого кадра).
        let had_ime = platform_state.has_ime_cmds();

        let had_events = !backend_events.is_empty() || self.input_state.back_pressed || had_ime;

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
                        platform_state,
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

        // --- Шаг 2.5: IME-команды из Kotlin InputConnection ---
        //
        // JNI-обработчики (`ime_jni`) кладут `ImeCmd` в потокобезопасную очередь
        // `PlatformState.ime_cmds`. Здесь забираем очередь и конвертируем команды
        // в egui-события (`Event::Text`, `Event::Ime(ImeEvent::...)`).
        // Никакого `TextEvent`/`setTextInputState`/`textInputState()` больше нет.
        let ime_cmds = platform_state.take_ime_cmds();

        // Системный Back закрыл IME на Kotlin-стороне: сбрасываем владельца
        // клавиатуры и снимаем egui-фокус с активного поля. Иначе поле остаётся
        // фокусным и владельцем, повторный тап не вызовет `gained_focus`, и
        // клавиатура не откроется заново. После surrender_focus поле получит
        // `lost_focus` на следующем кадре → TextEdit уберёт owner + publish_blur,
        // а следующий тап полноценно пере-покажет клавиатуру.
        if platform_state.take_ime_back_hidden() {
            log::info!("LOOP: системный Back закрыл IME — снимаем фокус");
            egui_ctx.data(|d| {
                if let Some(kb) = d.get_temp::<KeyboardController>(keyboard_controller_id()) {
                    *kb.owner_slot().write().unwrap() = None;
                }
            });
            egui_ctx.memory_mut(|m: &mut egui::Memory| {
                if let Some(focused_id) = m.focused() {
                    m.surrender_focus(focused_id);
                }
            });
        }

        if !ime_cmds.is_empty() {
            log::info!("LOOP: IME-команд на этом кадре: {}", ime_cmds.len());
            use crate::ime_service::{translate_legacy, ImeActionEvent, ImeEvent, ImeService};
            let mut svc_processed = 0usize;
            let mut dropped = 0usize;
            let mut total_egui_evs = 0usize;
            for cmd in ime_cmds {
                let Some(svc_cmd) = translate_legacy(&cmd) else {
                    dropped += 1;
                    continue; // no-op команда (SetSelection/PrivateCommand и т.п.)
                };
                svc_processed += 1;
                for ev in self.ime_service.apply(svc_cmd) {
                    match ev {
                        ImeEvent::Action(ImeActionEvent::Done) => {
                            // Done / Search / Go — завершить редактирование.
                            log::info!("LOOP: IME Done — закрываем клавиатуру");
                            crate::ime_jni::hide_soft_input_jni(
                                platform_state.vm_ptr(),
                                platform_state.activity_ptr(),
                            );
                            // Снимаем egui-фокус с активного поля (как в прежнем пути).
                            egui_ctx.memory_mut(|m: &mut egui::Memory| {
                                if let Some(focused_id) = m.focused() {
                                    m.surrender_focus(focused_id);
                                }
                            });
                        }
                        ImeEvent::Action(ImeActionEvent::Next) => {
                            // Next — перейти к следующему TextEdit. Переключение
                            // откладываем (см. `move_focus_next_due` ниже).
                            log::info!("LOOP: IME Next — отложенное переключение фокуса");
                            self.move_focus_next_pending = true;
                            self.move_focus_next_due = false;
                        }
                        other => {
                            // Текстовые операции доставляются в egui в этом кадре.
                            let e = crate::ime_service::to_egui_event(&other);
                            log::info!(
                                "LOOP: IME-ev {:?} -> egui {:?} | focused={:?}",
                                other,
                                e,
                                egui_ctx.memory(|m| m.focused())
                            );
                            total_egui_evs += 1;
                            self.input_state.events.push(e);
                        }
                    }
                }
            }
            log::info!(
                "LOOP: и того за кадр: команд={} svc_processed={} dropped={} egui_evs={}",
                svc_processed + dropped,
                svc_processed,
                dropped,
                total_egui_evs
            );
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
                egui_ctx,
            );
        }

        // --- Шаг 7: проверка уведомлений от data layer ---
        let had_notify = if let Some(ref mut ctx) = self.rt_ctx {
            ctx.check()
        } else {
            false
        };

        // --- Шаг 8: рендеринг ---
        //
        // Если были события от платформы или сигнал от data layer —
        // рендерим немедленно, игнорируя FPS-ограничение (target_dt).
        // Это гарантирует, что после клика навигация/действие отрабатывает
        // без задержки на ожидание следующего таймерного тика.
        //
        // В простое (had_events=false, had_notify=false) FPS-ограничение
        // работает как обычно — не чаще target_dt (60 FPS).
        let now = Instant::now();
        let dt_ok = now.duration_since(self.last_frame) >= target_dt;

        if dt_ok || had_events || had_notify {
            self.last_frame = now;

            let (w, h) = backend.window_size();
            if w == 0 || h == 0 {
                return true;
            }

            let pp = egui_ctx.pixels_per_point();

            // Получаем insets для этого кадра
            let insets = get_current_insets(backend, pp, w, h);

            // ── Формируем события кадра ──
            //
            // Обычные события (touch/pointer) и текстовые операции IME уже лежат
            // в `input_state.events`: сервис ввода (шаг 2.5) применил команды и
            // отдал буферные операции (Insert/Replace/Delete), которые переведены
            // в egui-события здесь (см. `ime_service::to_egui_event`).
            let events_for_frame = std::mem::take(&mut self.input_state.events);
            let num_events = events_for_frame.len();

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
            // log::info!("LOOP: frame() вернулся"); // спам, закомментирован
            //
            // ── Управление видимостью клавиатуры ──
            // Показ/скрытие ЗДЕСЬ не выполняется. Источник истины — `TextEdit`
            // (ui-слой): показ по `gained_focus`/становлению владельцем, скрытие
            // по `lost_focus`, пока виджет был владельцем (`KeyboardController`).
            // Раньше здесь была hide-ветка по `!platform_output.ime`, но она
            // конфликтовала с показом: в первый кадр фокуса egui ещё не
            // выставляет `platform_output.ime`, и loop.rs закрывал только что
            // показанную клавиатуру (логи `ime неактивна — скрыть` сразу после
            // `KeyboardController.show()`). Это ломало повторное открытие.
            // Скрытие по системным событиям (IME Done, системный Back) и так
            // сделано выше, в обработке `ime_cmds`/`ime_back_hidden`.
            //
            // ── Cursor rect для candidate window IME ──
            // Пока клавиатура активна, каждый кадр передаём прямоугольник
            // курсора (экранные px) в Kotlin для позиционирования кандидатов.
            if let Some(ime_out) = &full_output.platform_output.ime {
                let c = ime_out.cursor_rect;
                let to_px = |v: f32| (v * pp).round() as i32;
                crate::ime_jni::update_cursor_rect_jni(
                    platform_state.vm_ptr(),
                    platform_state.activity_ptr(),
                    to_px(c.min.x),
                    to_px(c.min.y),
                    to_px(c.max.x),
                    to_px(c.max.y),
                );
            }

            // ── Переключение фокуса по IME_ACTION_NEXT ──
            //
            // Текстовые операции (Insert/Replace) применяются к полю в этом же
            // кадре (сервис ввода отдал их в `events` синхронно, шаг 2.5). Чтобы
            // накопленный текст гарантированно попал в ТЕКУЩЕЕ поле (а не в новое
            // после переключения фокуса), Next откладывается на один кадр:
            // `move_focus_next_due` на первом кадре после Next выставляется в true,
            // и только на следующем переключаем фокус.
            if self.move_focus_next_pending {
                if !self.move_focus_next_due {
                    // Первый кадр после Next — preedit ещё доставится в этом кадре.
                    self.move_focus_next_due = true;
                } else {
                    // preedit уже применён к текущему полю — можно переключать.
                    self.move_focus_next_pending = false;
                    self.move_focus_next_due = false;
                    let current = egui_ctx.memory(|m| m.focused());
                    let next = egui_ctx.data(|d| {
                        match d.get_temp::<KeyboardController>(keyboard_controller_id()) {
                            Some(kb) => {
                                let list = kb.registry_slot().read().unwrap().clone();
                                current.and_then(|c| next_ime_field_after(&list, c))
                            }
                            None => None,
                        }
                    });
                    if let Some(id) = next {
                        egui_ctx.memory_mut(|m| m.request_focus(id));
                        log::info!("LOOP: IME Next -> фокус на {:?}", id);
                    }
                }
            }

            if app_instance.request_destroy() {
                self.destroy_requested = true;
                return true;
            }

            // Запоминаем, когда egui хочет следующий кадр
            let new_delay = full_output
                .viewport_output
                .get(&ViewportId::ROOT)
                .map(|v| v.repaint_delay)
                .unwrap_or(Duration::ZERO);

            // Запоминаем, когда egui хочет следующий кадр.
            //
            // НЕ форсируем repaint_delay=0 по had_events: при постоянном потоке
            // событий от активной клавиатуры это превращает цикл в busy loop.
            // Полагаемся на egui: если экран не изменился — repaint_delay будет
            // большим, и цикл заблокируется в poll до следующего реального события.
            self.repaint_delay = new_delay;

            // // Спам-лог (каждый кадр) — закомментирован.
            // log::info!(
            //     "LOOP: repaint_delay = {:?} (had_events={}, had_notify={}, событий в кадре={})",
            //     self.repaint_delay,
            //     had_events,
            //     had_notify,
            //     num_events,
            // );

            // Синхронизируем clear color с темой Application
            // После frame() egui-стиль уже содержит panel_fill, установленный
            // Application через MaterialTheme::apply().
            {
                let style = egui_ctx.style_of(egui::Theme::Light).as_ref().clone();
                let bg = style.visuals.panel_fill;
                backend.platform_state().set_clear_color_from(bg);

                // Определяем тему по яркости фона и обновляем системные бары
                // только при реальной смене темы (не каждый кадр)
                let is_dark = (bg.r() as u32) + (bg.g() as u32) + (bg.b() as u32) < 384;
                let theme = if is_dark {
                    egui_android_platform::SystemTheme::Dark
                } else {
                    egui_android_platform::SystemTheme::Light
                };
                if self.last_theme != Some(theme) {
                    self.last_theme = Some(theme);
                    backend.set_theme_override(Some(theme));
                }
            }

            // Рендеринг через GraphicsPipeline
            // log::info!("LOOP: render_frame begin (w={} h={})", w, h); // спам
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
                // log::info!("LOOP: render_frame end success={}", success); // спам
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
