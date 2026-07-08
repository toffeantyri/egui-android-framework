//! Маршруты навигации для showcase-приложения.

/// Маршрут (экран) в приложении.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Route {
    /// Главный экран со списком демо.
    Home,
    /// Виджеты (Text, Button, Spacer).
    Widgets,
    /// Модификаторы (padding, size, background, etc).
    Modifiers,
    /// Контейнеры (Column, Row, Stack, LazyColumn).
    Containers,
    /// Темы (light/dark, палитра).
    Themes,
    /// Локальное состояние (remember).
    State,
    /// Анимации.
    Animations,
    /// Новая Modifier value type.
    ModifierValue,
    /// Вложенная навигация (экран 1 со вложенными A, B, C).
    Nested,
    /// Вложенные экраны в Nested.
    NestedA,
    NestedB,
    NestedC,
}

impl Route {
    /// Название экрана для отображения.
    pub fn title(&self) -> &str {
        match self {
            Route::Home => "Главная",
            Route::Widgets => "Виджеты",
            Route::Modifiers => "Модификаторы",
            Route::Containers => "Контейнеры",
            Route::Themes => "Темы",
            Route::State => "Локальное состояние",
            Route::Animations => "Анимации",
            Route::ModifierValue => "Modifier Value",
            Route::Nested => "Вложенная навигация",
            Route::NestedA => "Nested A",
            Route::NestedB => "Nested B",
            Route::NestedC => "Nested C",
        }
    }

    /// Описание экрана.
    pub fn description(&self) -> &str {
        match self {
            Route::Home => "Список демо-экранов",
            Route::Widgets => "Text, Button, Spacer, Icon",
            Route::Modifiers => "padding, size, background, align, clickable",
            Route::Containers => "Column, Row, Stack, LazyColumn",
            Route::Themes => "MaterialTheme, ColorPalette, Typography",
            Route::State => "remember, RememberState",
            Route::Animations => "AnimatedVisibility, Fade, Slide",
            Route::ModifierValue => "Новая Modifier value type system",
            Route::Nested => "Демо вложенной навигации с BackPressed",
            Route::NestedA => "Первый вложенный экран",
            Route::NestedB => "Второй вложенный экран",
            Route::NestedC => "Третий вложенный экран",
        }
    }
}
