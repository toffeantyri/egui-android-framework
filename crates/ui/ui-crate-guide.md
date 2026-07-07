# Руководство и архитектурный контракт UI-слоя `egui-android-ui`

Этот документ объединяет две роли:
1. **Руководство** — как пользоваться UI-слоем, примеры, API
2. **Архитектурный контракт** — жёсткие правила, которые должен соблюдать каждый виджет, модификатор и контейнер

---

## 1. Общие принципы виджетов (контракт)

### 1.1 Виджет — чистая декларация

Виджет не содержит бизнес-логики, не читает State, не вызывает Store, не знает про Android.

Он только:
- описывает структуру UI,
- измеряет свой контент,
- рисует себя,
- диспатчит сообщения через `Dispatcher<M>`.

### 1.2 Виджет не имеет побочных эффектов

Метод `render(ui, dispatch)`:
- не должен изменять глобальное состояние,
- не должен выполнять async-операции,
- не должен вызывать внешние сервисы,
- не должен изменять State напрямую.

### 1.3 Виджет управляется модификаторами и Constraints

Все изменения поведения (размер, фон, кликабельность, выравнивание, анимации)
должны происходить через модификаторы.

Размер виджета определяется его желаемым размером (wrap-content) и
`Constraints`, переданными от родителя (через `UiWrapper`). Виджет
вызывает `allocate_space_with_sense`, который `clamp`'ит желаемый размер
к constraints.

### 1.4 Виджет не знает про `egui::Ui`

Виджет получает `&mut UiWrapper`, а не `&mut egui::Ui`. Все вызовы к egui
работают через `Deref<Target = egui::Ui>`.

---

## 2. Constraints

`Constraints` — структура, задающая min/max ширину и высоту для виджета.
Аналог `Constraints` в Jetpack Compose.

**Расположение:** `egui_android_core::Constraints`
**Re-export:** `egui_android_ui::Constraints`

```rust
pub struct Constraints {
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
}
```

### 2.1 Создание

| Метод | Описание |
|---|---|
| `Constraints::unconstrained()` | 0..INF по всем осям (по умолчанию) |
| `Constraints::exact(w, h)` | min == max (точный размер) |
| `Constraints::ranged(min_w, max_w, min_h, max_h)` | Диапазон размеров |

### 2.2 Использование

```rust
let c = Constraints::exact(200.0, 100.0);
let clamped = c.clamp_size(egui::vec2(50.0, 30.0));
// clamped = (200.0, 100.0) — clamp к min
```

### 2.3 Отличие от Compose

В Compose constraints приходят от родителя к детям при каждом layout.
В egui constraints хранятся в `UiWrapper` и передаются через `Context::data()`
(чтобы пережить `Frame::show`, `ScrollArea::show` и другие egui-обёртки).

---

## 3. UiWrapper

`UiWrapper` — обёртка над `egui::Ui`, добавляющая поддержку `Constraints`.
Реализует `Deref<Target = egui::Ui>` для полной совместимости.

**Расположение:** `egui_android_core::UiWrapper`
**Re-export:** `egui_android_ui::UiWrapper`

```rust
pub enum UiWrapper<'a> {
    Borrowed(&'a mut egui::Ui, Constraints),
    Owned(egui::Ui, Constraints),
}
```

### 3.1 Создание

| Метод | Описание |
|---|---|
| `UiWrapper::new(ui, constraints)` | Создать с явными constraints |
| `UiWrapper::new_unconstrained(ui)` | Создать, читает constraints из Context (если были) |

### 3.2 Основные методы

| Метод | Описание |
|---|---|
| `.constraints()` | Получить текущие constraints |
| `.set_constraints(c)` | Установить constraints (поле + Context) |
| `.allocate_space(size)` | Alloc'ит `clamp_size(size)` с `Sense::hover()` |
| `.allocate_space_with_sense(size, sense)` | Alloc'ит `clamp_size(size)` с указанным sense |
| `UiWrapper::new_child(ui, builder)` | Создать child UiWrapper с наследованием constraints |
| `UiWrapper::new_child_with_constraints(ui, builder, c)` | Создать child UiWrapper с новыми constraints |

### 3.3 Хранение constraints (гибридный подход)

Constraints хранятся в двух местах:
1. **Поле `constraints`** — type-safe, быстрый доступ
2. **`Context::data()`** — записываются с ключом `"ui_wrapper_cx"`, чтобы пережить `Frame::show`, `ScrollArea::show`

---

## 4. Layout-pipeline (контракт)

Каждый виджет обязан реализовывать трёхфазный layout, аналогичный Jetpack Compose:

### 4.1 Measure phase

Виджет обязан:
- измерить свой контент,
- получить точный размер (`content_size`),
- учесть модификаторы размера (`Modifier.size`, `fill_max_width`, `fill_max_height`),
- учесть padding/background/align.

### 4.2 Layout phase

Виджет обязан:
- вызывать `ui.allocate_space_with_sense(content_size, sense)` или `ui.allocate_space(content_size)`
  (эти методы учитывают `Constraints` через `clamp_size`),
- **не использовать `available_size()` как размер по умолчанию**.

### 4.3 Paint phase

Виджет обязан:
- рисовать контент строго внутри выделенной области,
- использовать `ui.painter()` только после layout.

---

## 5. Widget<M> трейт

Базовый трейт для всех виджетов. Принимает `&mut UiWrapper` вместо `&mut egui::Ui`.

**Расположение:** `egui_android_core::widget::Widget`
**Re-export:** `egui_android_ui::widgets::Widget`

```rust
pub trait Widget<M> {
    fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<M>);
}
```

### 5.1 Правила реализации (контракт)

1. **Виджет — чистая декларация.** Не содержит бизнес-логики, не читает State, не вызывает Store.
2. **Нет побочных эффектов.** `render` не изменяет глобальное состояние, не вызывает async.
3. **Layout-pipeline:** measure → allocate → paint.
4. **Не использовать `available_size()` как размер по умолчанию.**
5. **Использовать `allocate_space_with_sense`** для alloc с учётом constraints.
6. **Не использовать `allocate_exact_size`** напрямую — он не учитывает constraints.

```rust
// Правильно: alloc с учётом constraints
let (rect, response) = ui.allocate_space_with_sense(
    desired_size,
    egui::Sense::click(),
);

// Неправильно: alloc в обход constraints
// let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
```

---

## 6. Виджеты

### 6.1 Button

**Расположение:** `egui_android_ui::widgets::Button`

```rust
Button::new("Текст")
    .on_click(Msg::Clicked)
    .padding(8.0)
    .render(ui, dispatch);
```

**Размер:**
- По умолчанию — wrap-content (текст + внутренний padding 12px гор, 8px верт, высота min 48px)
- Full-width — только через модификатор:
  ```rust
  Button::new("Full-width")
      .on_click(Msg::Clicked)
      .modifier(Modifier::new().fill_max_width().padding(8.0))
      .render(ui, dispatch);
  ```
- Фиксированный размер — через модификатор:
  ```rust
  Button::new("200x48")
      .size(200.0, 48.0)
      .render(ui, dispatch);
  ```

**Механизм растяжения:**
`Button::render` вызывает `ui.allocate_space_with_sense(desired_size, Sense::click())`.
Если `Constraints::min_width > desired_width` (установлен `FillMaxWidth`),
`allocate_space_with_sense` вернёт rect шириной `min_width`. Кнопка растягивается.

**Обработка клика:**
1. Сначала диспатчится сообщение (MVI-поток)
2. Затем вызывается closure (локальное UI-действие)

#### Контракт для Button

- **Wrap-content по умолчанию** — кнопка измеряет текст через galley и не растягивается без модификатора.
- **Full-width только через модификатор** — `.size(...)`, `.fill_max_width()`.
- **Многострочный текст** — Button использует multiline layout, учитывая `available_width()`.

### 6.2 Text

**Расположение:** `egui_android_ui::widgets::Text`

```rust
Text::new("Текст").render(ui, dispatch);
Text::new("Текст").font_size(24.0).selectable(true).render(ui, dispatch);
```

**Размер:**
- По умолчанию — wrap-content (размер galley)
- Non-selectable: использует `LayoutJob` с переносом строк
- Selectable: использует `ui.label()` (стандартный egui)
- Растяжение через `allocate_space()` — если `Constraints::min_width > text_width`, alloc'ит min_width

#### Контракт для Text

- **Текст не должен растягивать контейнеры** — использует wrap (перенос строк), учитывает `max_width`.
- **Selectable/non-selectable** — имеют разный layout-pipeline (осознанное архитектурное решение).

### 6.3 Spacer

**Расположение:** `egui_android_ui::widgets::Spacer`

```rust
Spacer::new(16.0).render(ui, dispatch);        // вертикальный отступ
Spacer::width(100.0).render(ui, dispatch);       // горизонтальный
Spacer::size(100.0, 50.0).render(ui, dispatch);  // обе оси
```

Всегда использует `allocate_space(size)`. `fill_max_width` на Spacer не влияет.

#### Контракт для Spacer

- **Только allocate_exact_size** — не использует `add_space`.
- **Не ломает Row/Column** — корректно работает с модификаторами.

### 6.4 Icon

**Расположение:** `egui_android_ui::widgets::Icon`

```rust
Icon::new(egui::Image::new("...")).render(ui, dispatch);
```

Использует `ui.add(self.icon.clone())`. `fill_max_width` не поддерживается.

---

## 7. Модификаторы

### 7.1 Новая система (Modifier value type)

**Расположение:** `egui_android_ui::modifier::Modifier`

```rust
Modifier::new()
    .fill_max_width()
    .padding(16.0)
    .background(egui::Color32::DARK_GRAY)
    .clickable(Msg::Clicked)
```

#### Size constraints (контракт)

| Модификатор | Constraints на child_ui | Описание |
|---|---|---|
| `.fill_max_width()` | `Constraints::ranged(available.x, available.x, 0, INF)` | min_width = available.x |
| `.fill_max_size()` | `Constraints::exact(available.x, available.y)` | exact по обеим осям |
| `.width(w)` | `Constraints::ranged(w, w, 0, INF)` | exact по ширине |
| `.height(h)` | `Constraints::ranged(0, INF, h, h)` | exact по высоте |
| `.wrap_content_width()` | Измеряет контент через scope | alloc по реальному размеру |
| `.wrap_content_size()` | Аналогично по обеим осям | alloc по реальному размеру |
| `.width_in(min, max)` | alloc'ит `(clamp(w, min, max), available.y)` | диапазон ширины |
| `.height_in(min, max)` | alloc'ит `(available.x, clamp(h, min, max))` | диапазон высоты |

#### Appearance

| Модификатор | Описание |
|---|---|
| `.background(color)` | Заливка фона через `Frame::NONE.fill(color)` |
| `.border(width, color)` | Рамка через `Frame::NONE.stroke(...)` |
| `.rounding(radius)` | Скругление углов |
| `.alpha(a)` | Прозрачность (0.0..1.0) |
| `.clip(rounding)` | Обрезка содержимого |
| `.shadow(elevation)` | Имитация тени |

#### Interaction

| Модификатор | Описание |
|---|---|
| `.clickable(msg)` | Делает кликабельным, диспатчит msg |
| `.clickable_with(closure)` | Делает кликабельным, вызывает closure |

**Механизм Clickable (контракт):**
1. Рендерит контент в `child_ui` (один раз)
2. Измеряет `child_ui.min_size()`
3. Аллоцирует кликабельную область ровно по размеру контента
4. При клике диспатчит msg / вызывает closure

#### Animation (контракт)

| Модификатор | Описание |
|---|---|
| `.fade(opacity)` | Прозрачность через `multiply_opacity` |
| `.slide(direction, offset)` | Смещение контента |

### 7.2 Старая система (ModifierExt)

**Расположение:** `egui_android_ui::modifier::legacy::ModifierExt`

```rust
Text::new("Текст")
    .padding(8.0)
    .background(egui::Color32::RED)
    .clickable(())
    .render(ui, dispatch);
```

Старая система использует wrapper-структуры (`Padded`, `SizedWidget`, `Background`,
`Aligned`, `Clickable`, `ClickableWith`). Все они реализуют `Widget<M>` и делегируют
рендер внутреннему виджету.

**SizedWidget — контракт:** Создаёт child_ui с `Constraints::exact(width, height)`.

### 7.3 Порядок модификаторов (контракт)

Модификаторы применяются **в порядке добавления** (первый — самый внешний):

```rust
Modifier::new()
    .fill_max_width()       // 1. Резервирует всю ширину (constraints.min_width = available.x)
    .padding(16.0)          // 2. Внутри — отступ (Frame::inner_margin)
    .background(RED)        // 3. Фон рисуется поверх padding (Frame::fill)
    .clickable(Msg::Click)  // 4. Кликабельная область = размер контента
```

**Порядок имеет значение:**
- `.fill_max_width().padding(16.0)` — padding внутри полной ширины
- `.padding(16.0).fill_max_width()` — сначала padding, потом растяжение (может не дать эффекта)
- `.background(RED).padding(16.0)` — padding снаружи фона
- `.padding(16.0).background(RED)` — фон поверх padding

### 7.4 Контракт для модификаторов

1. **Модификатор — чистая функция над layout.** Не изменяет State, не диспатчит сообщения, не выполняет side-effects.
2. **Модификаторы не конфликтуют.** Комбинации `padding + background`, `size + clickable`, `align + background` работают детерминированно.
3. **Каждый модификатор рендерит контент ровно один раз.** Запрещён двойной рендер.
4. **Устанавливают Constraints на child_ui** (FillMaxWidth, FillMaxSize, Width, Height), а не вызывают `set_min_width/set_min_height`.

---

## 8. Контейнеры

### 8.1 Column

**Расположение:** `egui_android_ui::containers::Column`

```rust
Column::new()
    .spacing(8.0)
    .scrollable()
    .show(ui, dispatch, |ui, dispatch| {
        Text::new("Элемент 1").render(ui, dispatch);
        Text::new("Элемент 2").render(ui, dispatch);
    });
```

**Constraints на детей:** `Constraints::ranged(0, available.x, 0, INF)`

**Контракт:**
- Spacing по умолчанию — 8.0 (консистентно для всех контейнеров)
- Не принуждает детей растягиваться на всю доступную ширину/высоту
- Scrollable использует `auto_shrink([false, false])` — не сжимается по ширине
- Передаёт constraints с `max_width = available.x` чтобы fill_max_width работал

### 8.2 Row

```rust
Row::new(ui, dispatch, |ui, dispatch| {
    Text::new("Левый").render(ui, dispatch);
    Text::new("Правый").render(ui, dispatch);
});
```

**Constraints на детей:** `Constraints::ranged(0, INF, 0, available.y)`

**Контракт:**
- Spacing по умолчанию — 8.0
- Ширина детей — wrap-content (0..INF), высота — доступная
- `fill_max_width` в Row не имеет смысла (Row alloc'ит детям горизонтально)

### 8.3 Stack

```rust
Stack::new(ui, dispatch, |ui, dispatch| {
    Text::new("Фон").background(egui::Color32::BLUE).render(ui, dispatch);
    Text::new("Поверх").render(ui, dispatch);
});
```

**Constraints на детей:** `Constraints::ranged(0, inner_rect.width, 0, inner_rect.height)`
**Размер:** wrap-content (max размер детей)

**Контракт:**
- Wrap-content — не использует `ui.available_size()` как размер
- Измеряет всех детей, берёт max(child_size)
- alloc в родителе после рендера
- Из-за `FnOnce` ограничения — двойной проход (alloc + render)

### 8.4 LazyColumn

```rust
let items = vec![1, 2, 3];
LazyColumn::new(items, ui, dispatch, |item, ui, dispatch| {
    Text::new(format!("Элемент {}", item)).render(ui, dispatch);
});
```

**Constraints на детей:** `Constraints::ranged(0, available.x, 0, INF)`
**item_spacing по умолчанию:** 8.0

**Контракт:**
- Каждый элемент получает уникальный `push_id(index)` для корректной работы `remember()`
- Использует `ScrollArea::vertical()` с прокруткой

---

## 9. Как работает fill_max_width (полный цикл)

1. **Пользователь пишет:**
   ```rust
   Button::new("Кнопка")
       .modifier(Modifier::new().fill_max_width().padding(8.0))
       .render(ui, dispatch);
   ```

2. **Modifier::apply_recursive** обрабатывает узлы:
   - `FillMaxWidth` → создаёт child_ui с `Constraints::ranged(available.x, available.x, 0, INF)`
   - `PaddingAll(8.0)` → `Frame::NONE.inner_margin(8).show(child_ui, |ui| rest)`
   - `Button::render` → вызывается внутри Frame

3. **Button::render:**
   - Измеряет текст → `btn_width = text_width + 24`
   - Вызывает `ui.allocate_space_with_sense((btn_width, 48), Sense::click())`
   - `allocate_space_with_sense` → `constraints.clamp_size((btn_width, 48))`
   - `clamp_size` → `min_width = available.x > btn_width` → `(available.x, 48)`
   - alloc'ится rect шириной `available.x` — кнопка растянулась

4. **Column** alloc'ил `(available.x, 48)` в родителе → cursor сдвинулся на 48px

---

## 10. Анимации

### 10.1 AnimatedVisibility

```rust
AnimatedVisibility::new(visible, 0.3)
    .child(Text::new("Появляющийся текст"))
    .render(ui, dispatch);
```

Использует `animate_bool_with_time`. При `progress <= 0.0` не рендерит дочерний виджет.

### 10.2 Fade (через AnimationExt)

```rust
Text::new("Текст").fade(0.6).render(ui, dispatch);
```

Оборачивает контент в `ui.scope()` с `multiply_opacity(opacity)`.

### 10.3 Slide (через AnimationExt)

```rust
Text::new("Текст").slide(SlideDirection::Down, 20.0).render(ui, dispatch);
```

Смещает `max_rect` контента в указанном направлении.

### 10.4 Анимации как ModifierNode

Fade и Slide также доступны как узлы Modifier value type:
```rust
Modifier::new().fade(0.6).slide(SlideDirection::Down, 20.0)
```

**Контракт для анимаций:**
- Не меняют layout виджета
- Не меняют размер виджета
- Являются частью Modifier chain (Fade, Slide как ModifierNode)
- `AnimatedVisibility` — контейнер с состоянием, не модификатор

---

## 11. Тема

**Расположение:** `egui_android_ui::theme`

```rust
// Установить тему
MaterialTheme::light().apply(ctx);
MaterialTheme::dark().apply(ctx);

// Прочитать текущую тему
let theme = Theme::current(ctx);
let theme = Theme::current_from_ui(ui);
```

### 11.1 Theme

```rust
pub struct Theme {
    pub colors: ColorPalette,
    pub typography: Typography,
    pub shapes: Shapes,
}
```

### 11.2 ColorPalette (Material Design 3)

```rust
pub struct ColorPalette {
    pub primary: egui::Color32,
    pub on_primary: egui::Color32,
    pub secondary: egui::Color32,
    pub on_secondary: egui::Color32,
    pub background: egui::Color32,
    pub on_background: egui::Color32,
    pub surface: egui::Color32,
    pub on_surface: egui::Color32,
    pub error: egui::Color32,
    pub on_error: egui::Color32,
}
```

### 11.3 Typography

```rust
pub struct Typography {
    pub display_large: egui::FontId,
    pub display_medium: egui::FontId,
    pub display_small: egui::FontId,
    pub headline_large: egui::FontId,
    pub headline_medium: egui::FontId,
    pub headline_small: egui::FontId,
    pub title_large: egui::FontId,
    pub title_medium: egui::FontId,
    pub title_small: egui::FontId,
    pub body_large: egui::FontId,
    pub body_medium: egui::FontId,
    pub body_small: egui::FontId,
    pub label_large: egui::FontId,
    pub label_medium: egui::FontId,
    pub label_small: egui::FontId,
}
```

### 11.4 Shapes

```rust
pub struct Shapes {
    pub small: egui::CornerRadius,
    pub medium: egui::CornerRadius,
    pub large: egui::CornerRadius,
}
```

---

## 12. remember — локальное состояние

**Расположение:** `egui_android_ui::remember`

```rust
let count = remember(ui, "counter", || 0i32);
count.set(42);
count.modify(|c| *c += 1);
println!("{}", *count.get());
```

Хранится в `egui::Context::data()` как `Arc<RwLock<T>>`. Клонируется с разделением одного состояния.

**Методы RememberState:**
- `.get()` → `RwLockReadGuard<T>` — читает значение
- `.set(value)` — устанавливает новое значение
- `.modify(|v| ...)` — изменяет через замыкание

---

## 13. Layout-модель (сравнение с Compose)

### Compose
```
Родитель → Constraints(min, max) → Ребёнок
Ребёнок → Measure(size) → Родитель
Родитель → Place(x, y) → Ребёнок
```

### egui-android-ui
```
UiWrapper → Constraints (min_width, max_width, min_height, max_height)
→ alloc через allocate_space_with_sense → clamp к Constraints
→ Parent alloc'ит rect по clamped size
```

### 13.1 Ограничения egui

1. **Constraints не передаются автоматически через `Frame::show`** — поэтому используется `Context::data()`.
2. **`available_size().x` в Column не меняется** — Column alloc'ит фиксированную ширину. Проверка fill_max_width по ширине через `available_size().x` невозможна.
3. **`min_rect()` обнуляется внутри `Frame::show`** — поэтому виджеты проверяют не `min_rect().width()`, а используют `allocate_space` с clamp к constraints.

---

## 14. Сравнение старой и новой системы модификаторов

| Аспект | Старая (ModifierExt) | Новая (Modifier value type) |
|---|---|---|
| Тип | Wrapper-структуры на каждый модификатор | Единый `Modifier<M>` с вектором узлов |
| Порядок | Через вложенность типов | Через порядок добавления в цепочке |
| Constraints | Не поддерживает | Поддерживает через `UiWrapper` |
| Производительность | N alloc'ов на N модификаторов | N alloc'ов (аналогично) |
| Гибкость | Каждый модификатор — отдельный тип | Можно передавать как value |

Обе системы работают параллельно и совместимы:
```rust
// Старая
Text::new("...").padding(8.0).background(RED).render(ui, dispatch);

// Новая
Text::new("...")
    .modifier(Modifier::new().padding(8.0).background(RED))
    .render(ui, dispatch);

// Смешанная
Button::new("...")
    .on_click(Msg::Click)
    .modifier(Modifier::new().fill_max_width().padding(8.0))
    .render(ui, dispatch);
```

---

## 15. Clickable / ClickableWith (контракт)

1. **Размер клика — размер контента.** Clickable не использует `available_size()`.
2. **Никакого двойного рендера.** Контент рендерится один раз в child_ui.
3. **Не меняет layout контейнера.** Не растягивает Row/Column.
4. **Response соответствует отрендеренной области.** clickable-область визуально и логически совпадает с alloc'нутой областью.

---

## 16. Тесты (контракт)

Каждый виджет обязан иметь тесты:
- wrap-content,
- модификаторы,
- clickable,
- nested containers,
- LazyColumn,
- Stack,
- multiline text,
- fill_max_width (в Column, с Text, с Button),
- fill_max_size,
- SizedWidget,
- Height/Width через constraints,
- negative modifiers (ошибки).

**Тесты находятся в:** `crates/ui/tests/widget_tests.rs` (121 тест), `crates/ui/tests/remember_tests.rs` (14 тестов)

```bash
cargo test -p egui-android-ui
cargo test --workspace
```

---

## 17. Итог

Этот документ — спецификация и руководство для `egui-android-ui`.
Каждый виджет, модификатор и контейнер обязан соблюдать контракт,
описанный в секциях «контракт».

Если что-то нарушает контракт — это архитектурная ошибка.
