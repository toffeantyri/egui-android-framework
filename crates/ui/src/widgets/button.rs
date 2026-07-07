//! Виджет [`Button`] — кликабельная кнопка с wrap-content по умолчанию.
//!
//! Диспатчит сообщение при клике или вызывает closure.
//! Использует builder pattern для задания `on_click` / `on_click_with`.
//!
//! # Размер
//!
//! По умолчанию кнопка занимает размер, достаточный для текста (wrap-content).
//! Full-width (на всю ширину контейнера) — только через модификатор:
//! `.modifier(Modifier::new().fill_max_width())` или через `.padding()` + `.background()`.
//! Высота по умолчанию вычисляется по тексту + внутренний padding.
//! Можно переопределить через [`Button::height`].
//! Текст выравнивается по центру кнопки.
//! Вся область кнопки реагирует на клик.
//!
//! # Визуальная обратная связь
//!
//! При нажатии (Down) кнопка меняет цвет фона на более яркий/тёмный.
//! Это даёт пользователю тактильный отклик, аналогичный ripple в Jetpack Compose.
//!
//! Цвета по умолчанию:
//! - Обычное состояние: `(0, 128, 255)` — синий
//! - Нажатое состояние: `(255, 120, 0)` — оранжевый
//!
//! Можно переопределить через [`Button::colors`].
//!
//! # Пример
//!
//! ```ignore
//! // Кнопка с MVI-сообщением (wrap-content по умолчанию)
//! Button::new("Нажми меня")
//!     .on_click(Msg::Clicked)
//!     .render(ui, dispatch);
//!
//! // Кнопка на всю ширину через модификатор
//! Button::new("Full-width")
//!     .on_click(Msg::Clicked)
//!     .modifier(Modifier::new().fill_max_width())
//!     .render(ui, dispatch);
//!
//! // Кнопка с кастомными цветами
//! Button::new("Кастом")
//!     .on_click(Msg::Clicked)
//!     .colors(
//!         egui::Color32::from_rgb(0, 200, 100),  // обычный
//!         egui::Color32::from_rgb(0, 255, 150),  // нажатый
//!     )
//!     .render(ui, dispatch);
//! ```

use egui_android_core::{widget::Widget, Dispatcher, UiWrapper};

/// Цвета кнопки для различных состояний.
pub struct ButtonColors {
    /// Цвет фона в обычном состоянии.
    pub normal: egui::Color32,
    /// Цвет фона при нажатии (Down).
    pub pressed: egui::Color32,
}

impl Default for ButtonColors {
    fn default() -> Self {
        Self {
            normal: egui::Color32::from_rgb(0, 128, 255),
            pressed: egui::Color32::from_rgb(255, 120, 0),
        }
    }
}

/// Виджет кнопки.
///
/// При клике:
/// 1. Диспатчит сообщение, заданное через [`Button::on_click`] (если задано).
/// 2. Вызывает closure, заданный через [`Button::on_click_with`] (если задан).
///
/// Оба обработчика могут быть заданы одновременно — сначала диспатчится
/// сообщение, затем вызывается closure.
///
/// По умолчанию кнопка занимает размер текста + внутренний padding (wrap-content).
/// Full-width — только через модификатор (`.fill_max_width()` или `.size(...)`).
/// Выравнивает текст по центру.
/// Вся область кнопки реагирует на клик.
///
/// Встроенная визуальная обратная связь: при нажатии цвет фона меняется.
pub struct Button<M> {
    text: String,
    on_click_msg: Option<M>,
    on_click_callback: Option<Box<dyn Fn(&UiWrapper, &Dispatcher<M>)>>,
    height: f32,
    colors: ButtonColors,
}

impl<M: 'static> Button<M> {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            on_click_msg: None,
            on_click_callback: None,
            height: 48.0,
            colors: ButtonColors::default(),
        }
    }

    /// Установить сообщение, которое будет диспатчиться при клике.
    ///
    /// Сообщение проходит через стандартный MVI-поток
    /// (Dispatcher → Component::handle → Data Layer → StateStore).
    pub fn on_click(mut self, msg: M) -> Self {
        self.on_click_msg = Some(msg);
        self
    }

    /// Установить closure, который будет вызван при клике.
    ///
    /// Используется для локальных UI-действий (например, изменение `remember`),
    /// которые не требуют MVI-потока.
    ///
    /// Closure получает `&UiWrapper` и `&Dispatcher<M>` — может читать `remember`,
    /// модифицировать локальное состояние и/или диспатчить сообщения.
    ///
    /// # Пример
    ///
    /// ```ignore
    /// let count = remember(ui, "counter", || 0i32);
    /// Button::new("+1")
    ///     .on_click_with({
    ///         let count = count.clone();
    ///         move |_ui, _dispatch| {
    ///             count.modify(|c| *c += 1);
    ///         }
    ///     })
    ///     .render(ui, dispatch);
    /// ```
    pub fn on_click_with<F>(mut self, callback: F) -> Self
    where
        F: Fn(&UiWrapper, &Dispatcher<M>) + 'static,
    {
        self.on_click_callback = Some(Box::new(callback));
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Установить кастомные цвета для обычного и нажатого состояния.
    ///
    /// # Пример
    ///
    /// ```ignore
    /// Button::new("ОК")
    ///     .on_click(Msg::Ok)
    ///     .colors(
    ///         egui::Color32::from_rgb(0, 180, 80),   // обычный — зелёный
    ///         egui::Color32::from_rgb(0, 255, 120),  // нажатый — ярко-зелёный
    ///     )
    ///     .render(ui, dispatch);
    /// ```
    pub fn colors(mut self, normal: egui::Color32, pressed: egui::Color32) -> Self {
        self.colors = ButtonColors { normal, pressed };
        self
    }
}

impl<M: Clone + 'static> Widget<M> for Button<M> {
    fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<M>) {
        // Внутренний padding кнопки: 12px по горизонтали, 8px по вертикали
        const HPAD: f32 = 12.0;
        const VPAD: f32 = 8.0;

        let font_id = egui::FontId::proportional(18.0);
        let text_color = egui::Color32::WHITE;

        // Измеряем текст
        let galley = ui
            .painter()
            .layout_no_wrap(self.text.clone(), font_id, text_color);
        let text_size = galley.size();

        // Размер кнопки: wrap-content (текст + padding).
        // Если constraints.min_width > btn_width (от FillMaxWidth),
        // allocate_space_with_sense clamp'нет до min_width.
        let btn_height = self.height.max(text_size.y + VPAD * 2.0);
        let btn_width = text_size.x + HPAD * 2.0;
        let desired_size = egui::vec2(btn_width, btn_height);

        let (rect, response) = ui.allocate_space_with_sense(desired_size, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter_at(rect);

            // Выбираем цвет в зависимости от состояния нажатия
            let bg_color = if response.is_pointer_button_down_on() {
                self.colors.pressed
            } else {
                self.colors.normal
            };

            // Фон кнопки со скруглением
            painter.rect_filled(rect, 4.0, bg_color);

            // Текст по центру кнопки
            let text_pos = egui::pos2(
                rect.center().x - text_size.x / 2.0,
                rect.center().y - text_size.y / 2.0,
            );
            painter.galley(text_pos, galley, text_color);
        }

        if response.clicked() {
            // 1. Диспатчим сообщение (MVI-поток)
            if let Some(msg) = &self.on_click_msg {
                dispatch.dispatch(msg.clone());
            }
            // 2. Вызываем closure (локальное UI-действие)
            if let Some(callback) = &self.on_click_callback {
                callback(ui, dispatch);
            }
        }
    }
}
