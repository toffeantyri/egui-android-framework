//! Маршруты навигации для showcase-приложения.

/// Основной маршрут (экран) в приложении.
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
            Route::BackCustom => "Back переключает цвет фона",
        }
    }
}

/// Маршруты вложенной навигации внутри NestedScreen.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum NestedRoute {
    A,
    B,
    C,
}

impl NestedRoute {
    pub fn title(&self) -> &str {
        match self {
            NestedRoute::A => "Экран A",
            NestedRoute::B => "Экран B",
            NestedRoute::C => "Экран C",
        }
    }
}

/// Единый enum для навигации в любой стек.
///
/// Аналог sealed class NavigableRoutes в Kotlin/Decompose.
/// Каждый вариант — это отдельный стек со своим Route enum.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum NavigableRoute {
    /// Навигация в корневой стек.
    Main(Route),
    /// Навигация во вложенный стек NestedScreen.
    Nested(NestedRoute),
}
