//! Маршруты навигации для showcase-приложения.

use serde::{Deserialize, Serialize};

/// Основной маршрут (экран) в приложении.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// Сообщения для вложенного стека навигации внутри NestedScreen.
///
/// Каждый стек имеет свой тип сообщений — это позволяет
/// `NestedScreen` обрабатывать их самостоятельно через `handle_dyn()`,
/// без участия `NavigationHost`.
///
/// Аналог sealed class в Kotlin/Decompose: каждый стек определяет
/// свои маршруты и сообщения. `NavigationHost` просто пробрасывает
/// сообщение активному компоненту через `ComponentNode::handle_dyn()`.
#[derive(Clone, Debug)]
pub enum NestedMsg {
    /// Перейти на вложенный экран.
    Navigate(NestedRoute),
    /// Вернуться назад (pop из вложенного стека).
    Back,
    /// Открыть слой 2.
    OpenLayer2,
}

/// Маршруты второго уровня вложенной навигации.
///
/// Демонстрирует, что каждый вложенный стек имеет свой enum маршрутов
/// и свой тип сообщений, независимый от других стеков.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NestedLayer2Route {
    X,
    Y,
}

impl NestedLayer2Route {
    pub fn title(&self) -> &str {
        match self {
            NestedLayer2Route::X => "Экран X",
            NestedLayer2Route::Y => "Экран Y",
        }
    }
}

/// Сообщения для второго уровня вложенной навигации.
///
/// Абсолютно независимый тип — не связан с `NestedMsg` или `RootMsg`.
/// `NestedLayer2Screen` обрабатывает их через свой `handle_dyn()`, а
/// `NavigationHost` просто пробрасывает их активному компоненту.
#[derive(Clone, Debug)]
pub enum NestedLayer2Msg {
    /// Перейти на вложенный экран.
    Navigate(NestedLayer2Route),
    /// Вернуться назад (pop из стека).
    Back,
}
