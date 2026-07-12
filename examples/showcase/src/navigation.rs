//! Маршруты навигации для showcase-приложения.

/// Маршрут (экран) в приложении.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Route {
    /// Главный экран со списком демо.
    Home,
    /// Виджеты (Text, Button, Spacer).
    Widgets,
    /// Контейнеры (Column, Row, Stack, LazyColumn).
    Containers,
    /// Темы (light/dark, палитра).
    Themes,
    /// Локальное состояние (remember).
    State,
    /// Анимации.
    Animations,
    /// Все модификаторы (padding, size, background, border и т.д.).
    ModifierValue,
    /// Вложенная навигация (экран 1 со вложенными A, B, C).
    Nested,
    /// Вложенные экраны в Nested.
    NestedA,
    NestedB,
    NestedC,
    /// Кастомная обработка Back: меняет цвет фона вместо pop.
    BackCustom,
}

impl Route {
    /// Название экрана для отображения.
    pub fn title(&self) -> &str {
        match self {
            Route::Home => "Главная",
            Route::Widgets => "Виджеты",

            Route::Containers => "Контейнеры",
            Route::Themes => "Темы",
            Route::State => "Локальное состояние",
            Route::Animations => "Анимации",
            Route::ModifierValue => "Модификаторы",
            Route::Nested => "Вложенная навигация",
            Route::NestedA => "Nested A",
            Route::NestedB => "Nested B",
            Route::NestedC => "Nested C",
            Route::BackCustom => "Кастомный Back",
        }
    }

    /// Описание экрана.
    pub fn description(&self) -> &str {
        match self {
            Route::Home => "Список демо-экранов",
            Route::Widgets => "Text, Button, Spacer, Icon",

            Route::Containers => "Column, Row, Stack, LazyColumn",
            Route::Themes => "MaterialTheme, ColorPalette, Typography",
            Route::State => "remember, RememberState",
            Route::Animations => "AnimatedVisibility, Fade, Slide",
            Route::ModifierValue => "padding, size, background, border, clip, shadow и т.д.",
            Route::Nested => "Демо вложенной навигации с BackPressed",
            Route::NestedA => "Первый вложенный экран",
            Route::NestedB => "Второй вложенный экран",
            Route::NestedC => "Третий вложенный экран",
            Route::BackCustom => "Back переключает цвет фона",
        }
    }
}
