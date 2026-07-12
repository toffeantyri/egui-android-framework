# egui-android-ui

**Compose-like UI для egui на Android.**

Крейт содержит готовые виджеты, контейнеры, модификаторы, анимации и темы
для построения интерфейса в стиле Jetpack Compose.

[![crates.io](https://img.shields.io/crates/v/egui-android-ui)](https://crates.io/crates/egui-android-ui)

## Проблема

egui предоставляет низкоуровневые примитивы. Для построения сложного UI
нужны Column, Row, Stack, LazyColumn, система модификаторов, темы, анимации.
Этот крейт реализует всё это поверх egui.

## Возможности

### Виджеты
- **Button** — с визуальным откликом при нажатии (pressed ≠ normal). `theme_colors()` — автоматический подбор pressed под любую тему. `colors(normal, pressed)` — полный контроль.
- **Text** — однострочный и многострочный, с выравниванием.
- **Spacer** — вертикальный отступ.
- **Icon** — изображение.

### Контейнеры
- **Column** — вертикальный layout (аналог Compose Column). Spacing настраивается. Scrollable.
- **Row** — горизонтальный layout (аналог Compose Row).
- **Stack** — наложение виджетов (аналог Compose Box). Двухфазный measure→layout. Wrap-content (consum = max children). Builder pattern со списком детей.
- **LazyColumn** — скроллируемый список.

### Модификаторы
- **Размер:** `width`, `height`, `width_in`, `height_in`, `fill_max_width`, `wrap_content_width`, `wrap_content_size`, `size`
- **Внешний вид:** `background`, `border`, `clip`, `shadow`, `alpha`
- **Расположение:** `padding`, `padding_edges`
- **Взаимодействие:** `clickable`, `clickable_with`
- **Анимации:** `fade`, `slide`
- **Alignment:** `align(Center)` — через метод виджета

### Анимации
- **Fade** — плавное появление/исчезновение
- **Slide** — движение в 4 направлениях
- **AnimatedVisibility** — Compose-like: fade + slide при show/hide
- **animate_value**, **animate_bool** — интерполяция между значениями

### Тема
- **Material Design 3** — полная палитра (primary, secondary, background, surface, error и on-варианты)
- **Typography** — 15 размеров от display_large до label_small
- **Shapes** — small, medium, large скругления
- Автоматическое определение системной темы (light/dark)

### Локальное состояние
- **`remember<T>()`** — аналог Compose `remember`. Хранит состояние между кадрами через `Arc<RwLock<T>>`. Методы `get()`, `set()`, `modify()` работают через `&self`.

## Пример

```rust
use egui_android_ui::{
    containers::{Column, Stack, Align},
    modifier::{Modifier, ModifierDsl},
    widgets::{Button, Text, Widget},
    remember,
    theme::Theme,
};

Column::new().show(ui, dispatch, |ui, dispatch| {
    // Тема
    let theme = Theme::current(ui.ctx());
    let c = &theme.colors;

    // Текст с модификаторами
    Text::new("Привет!")
        .modifier(Modifier::new().padding(8.0).background(c.primary))
        .render(ui, dispatch);

    // Кнопка с визуальным откликом
    Button::new("Нажми")
        .theme_colors(c.primary)
        .text_color(c.on_primary)
        .on_click(Msg::Clicked)
        .modifier(Modifier::new().fill_max_width().padding(8.0))
        .render(ui, dispatch);

    // Stack с наложением
    Stack::new()
        .add(Text::new("Фон").modifier(Modifier::new().background(c.surface)))
        .add_with_align(Text::new("Центр"), Align::Center)
        .show(ui, dispatch);

    // Локальное состояние
    let count = remember(ui, "cnt", || 0i32);
    Text::new(format!("{}", count.get())).render(ui, dispatch);
});
```

## Когда использовать

Подключайте `egui-android-ui`, если вы строите UI с помощью фреймворка.
Все виджеты, контейнеры и модификаторы доступны через `egui-android-framework`
(umbrella). Отдельный крейт нужен для:
- написания своих виджетов/контейнеров
- тестирования UI-компонентов
- минимизации зависимостей
