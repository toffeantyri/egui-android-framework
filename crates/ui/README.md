# egui-android-ui

UI слой: виджеты, модификаторы, контейнеры, анимации, темы, локальное состояние.

## Проблема

egui предоставляет примитивные UI-элементы, но для создания Compose-like интерфейсов не хватает:
- декларативных контейнеров (Column, Row, Stack) с автоматическим constraint propagation
- системы модификаторов (padding, background, clickable) без вложенности
- анимаций появления/исчезновения
- готовой темы Material Design 3
- локального состояния между кадрами (remember)

Этот крейт предоставляет всё перечисленное.

## Возможности

### Виджеты
- **`Button<M>`** — кнопка с `on_click(msg)` и `on_click_with(closure)` (оба можно комбинировать)
- **`Text`** — текст с модификаторами
- **`Spacer`** — вертикальный отступ
- **`Icon`** — изображение

### Контейнеры (Compose-like замыкания)
- **`Column`** — вертикальное расположение, spacing по умолч. 8.0
- **`Row`** — горизонтальное расположение, spacing по умолч. 8.0
- **`Stack`** — наложение виджетов
- **`LazyColumn`** — скроллируемый список

Все контейнеры принимают `&mut UiWrapper`, `&Dispatcher` и closure.
Передают детям constraints: `fill_max_width()` внутри Column растягивает на всю ширину.

### Модификаторы
`Modifier::new()` — value-тип, иммутабельные методы:
`.padding(N)`, `.background(C)`, `.fill_max_width()`, `.size(W, H)`,
`.clickable(msg)`, `.align(Align)`, `.clip()`, `.shadow(Shadow)`

Применяются через `ModifierDsl`: `.modifier(Modifier::new().padding(8.0).background(red))`

### Анимации
- **`AnimatedVisibility<M>`** — плавное появление/исчезновение с длительностью
- **`Fade<W,M>`** — прозрачность
- **`Slide<W,M>`** — смещение
- **`animate_value()` / `animate_bool()`** — хелперы интерполяции через `egui::Context`
- **`AnimationExt<M>`** — extension trait: `.fade(opacity)`, `.slide(direction, offset)`

### Темы
- **`Theme`** — установка/чтение через `egui::Context::data()`
- **`MaterialTheme::light()` / `MaterialTheme::dark()`** — Material Design 3 палитры
- **`ColorPalette`**, **`Typography`**, **`Shapes`** — составные части темы

### Локальное состояние
- **`remember(ui, key, || init)`** — создаёт/восстанавливает состояние между кадрами
- **`RememberState<T>`** — внутри `Arc<RwLock<T>>`, методы `get()`, `set()`, `modify()` принимают `&self`
- Работает внутри замыканий контейнеров

## Зависимости

- `egui` — GUI
- `egui-android-core` — `Widget<M>`, `UiWrapper`, `Dispatcher`

## Пример

```rust
use egui_android_ui::prelude::*;
use egui_android_core::UiWrapper;

fn my_view(state: &AppState, ui: &mut UiWrapper, dispatch: &Dispatcher<Msg>) {
    Column::new().show(ui, dispatch, |ui, dispatch| {
        Text::new("Заголовок")
            .modifier(Modifier::new().padding(16.0).background(egui::Color32::from_gray(40)))
            .render(ui, dispatch);

        Button::new("Нажми меня")
            .on_click(Msg::Clicked)
            .modifier(Modifier::new()
                .padding(8.0)
                .background(egui::Color32::from_rgb(0, 128, 255))
                .fill_max_width())
            .render(ui, dispatch);
    });
}
```
