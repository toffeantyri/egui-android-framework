# egui-android-ui

UI слой: виджеты, модификаторы, контейнеры, анимации, темы.

## Возможности

- **Виджеты:** Button, Text, Spacer, Icon
- **Контейнеры:** Column, Row, Stack, LazyColumn (Compose-like замыкания)
- **Модификаторы:** padding, size, background, align, clickable, clip, shadow
- **Анимации:** AnimatedVisibility, Fade, Slide, animate_value, animate_bool
- **Темы:** Material Design 3 (light/dark), ColorPalette, Typography, Shapes
- **remember** — локальное состояние между кадрами (Arc<RwLock<T>>)

## Пример

```rust
use egui_android_ui::prelude::*;

Column::new(ui, dispatch, |ui, dispatch| {
    Text::new("Заголовок")
        .padding(16.0)
        .background(Color32::from_gray(40))
        .render(ui, dispatch);

    Button::new("Нажми меня")
        .on_click(Msg::Clicked)
        .padding(8.0)
        .background(Color32::from_rgb(0, 128, 255))
        .render(ui, dispatch);
});
```
