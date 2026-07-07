# План миграции на `UiWrapper` с Constraints

## Обзор проблемы

**Текущая ситуация:**
- Виджеты (Button, Text) не растягиваются на всю ширину контейнера при `fill_max_width()`
- Причина: в egui нет Constraints как в Compose
- Текущий хак: виджет смотрит `ui.available_width()` и использует его, если больше wrap-content
- Нарушение контракта: виджет не должен знать про контейнер

**Цель:**
- Реализовать Compose-like Constraints через обёртку `UiWrapper`
- Родитель передаёт ограничения, дети обязаны их соблюдать
- Чистая архитектура, масштабируемость, тестируемость

---

## Архитектурное решение

### Что такое `UiWrapper`?

Обёртка над `egui::Ui`, которая:
1. Хранит `Constraints` (min/max width/height)
2. Передаёт constraints детям при создании child_ui
3. Виджеты читают constraints и используют их при alloc'е

### Почему именно такая реализация?

| Альтернатива | Минусы | Почему не подходит |
|--------------|--------|-------------------|
| Патч egui | Зависит от версии, не попадает в апстрим | При обновлении egui нужно переписывать |
| `Context::data()` | Неэффективно, не type-safe | Поиск по id, нет гарантий |
| `available_width()` hack | Нарушает контракт | Виджет знает про контейнер |
| **`UiWrapper`** | **Нужен рефакторинг** | **Чистая архитектура, масштабируемо** |

### Преимущества `UiWrapper`

1. **Масштабируемость**
   - Можно добавлять новые поля в `Constraints` без изменения виджетов
   - Можно добавлять новые методы в `UiWrapper` без изменения egui
   
2. **Тестируемость**
   - Можно мокать `UiWrapper` в тестах
   - Не нужно создавать реальный `egui::Ui`
   
3. **Независимость от egui**
   - Если перейдём на другой GUI (iced, slint) — обёртка останется
   - Меняется только внутренняя реализация
   
4. **Чистая архитектура**
   - Виджеты не знают про контейнер, только про constraints
   - Родитель управляет layout через constraints

---

## Детальный план миграции

### Фаза 1: Создание инфраструктуры (1 день)

#### 1.1 Создать `Constraints` struct

**Файл:** `crates/ui/src/constraints.rs`

```rust
/// Constraints for layout, similar to Jetpack Compose.
/// 
/// Parent passes constraints to children, children must respect them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Constraints {
    /// Minimum width in points
    pub min_width: f32,
    /// Maximum width in points
    pub max_width: f32,
    /// Minimum height in points
    pub min_height: f32,
    /// Maximum height in points
    pub max_height: f32,
}

impl Constraints {
    /// Create constraints with exact size
    pub fn exact(width: f32, height: f32) -> Self {
        Self {
            min_width: width,
            max_width: width,
            min_height: height,
            max_height: height,
        }
    }
    
    /// Create constraints with min/max ranges
    pub fn ranged(
        min_width: f32,
        max_width: f32,
        min_height: f32,
        max_height: f32,
    ) -> Self {
        Self {
            min_width,
            max_width,
            min_height,
            max_height,
        }
    }
    
    /// Create unconstrained constraints
    pub fn unconstrained() -> Self {
        Self {
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: 0.0,
            max_height: f32::INFINITY,
        }
    }
    
    /// Clamp size to constraints
    pub fn clamp_size(&self, size: egui::Vec2) -> egui::Vec2 {
        egui::vec2(
            size.x.clamp(self.min_width, self.max_width),
            size.y.clamp(self.min_height, self.max_height),
        )
    }
}

impl Default for Constraints {
    fn default() -> Self {
        Self::unconstrained()
    }
}
```

**Зачем:**
- Единый тип для constraints
- Легко расширять (можно добавлять поля)
- Type-safe

#### 1.2 Создать `UiWrapper`

**Файл:** `crates/ui/src/ui_wrapper.rs`

```rust
use std::ops::{Deref, DerefMut};
use crate::Constraints;

/// Wrapper over `egui::Ui` with Constraints support.
/// 
/// Implements `Deref<Target = egui::Ui>` for compatibility with existing code.
pub struct UiWrapper<'a> {
    ui: &'a mut egui::Ui,
    constraints: Constraints,
}

impl<'a> UiWrapper<'a> {
    /// Create new wrapper with constraints
    pub fn new(ui: &'a mut egui::Ui, constraints: Constraints) -> Self {
        Self { ui, constraints }
    }
    
    /// Get constraints
    pub fn constraints(&self) -> &Constraints {
        &self.constraints
    }
    
    /// Set constraints
    pub fn set_constraints(&mut self, constraints: Constraints) {
        self.constraints = constraints;
    }
    
    /// Create child UiWrapper with inherited constraints
    pub fn new_child(&mut self, builder: egui::UiBuilder) -> UiWrapper<'_> {
        let child_ui = self.ui.new_child(builder);
        UiWrapper {
            ui: child_ui,
            constraints: self.constraints,
        }
    }
    
    /// Create child UiWrapper with new constraints
    pub fn new_child_with_constraints(
        &mut self,
        builder: egui::UiBuilder,
        constraints: Constraints,
    ) -> UiWrapper<'_> {
        let child_ui = self.ui.new_child(builder);
        UiWrapper {
            ui: child_ui,
            constraints,
        }
    }
    
    /// Allocate space respecting constraints
    pub fn allocate_space(&mut self, desired_size: egui::Vec2) -> (egui::Rect, egui::Response) {
        let clamped_size = self.constraints.clamp_size(desired_size);
        self.ui.allocate_exact_size(clamped_size, egui::Sense::hover())
    }
    
    /// Allocate space with sense respecting constraints
    pub fn allocate_space_with_sense(
        &mut self,
        desired_size: egui::Vec2,
        sense: egui::Sense,
    ) -> (egui::Rect, egui::Response) {
        let clamped_size = self.constraints.clamp_size(desired_size);
        self.ui.allocate_exact_size(clamped_size, sense)
    }
}

impl<'a> Deref for UiWrapper<'a> {
    type Target = egui::Ui;
    
    fn deref(&self) -> &Self::Target {
        self.ui
    }
}

impl<'a> DerefMut for UiWrapper<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.ui
    }
}
```

**Зачем:**
- `Deref<Target = egui::Ui>` — совместимость с существующим кодом
- Не нужно дублировать API egui
- Можно добавлять свои методы поверх
- Constraints передаются детям автоматически

#### 1.3 Обновить `lib.rs`

**Файл:** `crates/ui/src/lib.rs`

```rust
pub mod constraints;
pub mod ui_wrapper;

pub use constraints::Constraints;
pub use ui_wrapper::UiWrapper;
```

---

### Фаза 2: Изменение Widget trait (1 день)

#### 2.1 Изменить `Widget` trait в `egui-android-core`

**Файл:** `crates/core/src/widget.rs`

```rust
use egui_android_ui::UiWrapper;
use crate::Dispatcher;

/// Base trait for all widgets.
/// 
/// Widget knows only about `UiWrapper` and `Dispatcher`.
/// Widget does NOT know about Store, Component, Reducer.
pub trait Widget<M> {
    /// Render widget in UI.
    /// 
    /// Can dispatch messages through `dispatch` at event time.
    fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<M>);
}
```

**Зачем:**
- Виджеты работают с `UiWrapper`, а не с `egui::Ui`
- Виджеты читают constraints из `UiWrapper`
- Чистая архитектура

**Проблема:**
- `egui-android-core` зависит от `egui-android-ui`? Нет, это циклическая зависимость!

**Решение:**
- Переместить `UiWrapper` и `Constraints` в `egui-android-core`
- Или создать новый крейт `egui-android-layout`

**Выбор:**
- Переместить в `egui-android-core` (проще, меньше изменений)

#### 2.2 Переместить `UiWrapper` и `Constraints` в `core`

**Файлы:**
- `crates/core/src/constraints.rs` (из `ui`)
- `crates/core/src/ui_wrapper.rs` (из `ui`)
- `crates/core/src/lib.rs` (обновить)

**Зачем:**
- Избежать циклической зависимости
- `core` — базовый крейт, все от него зависят

---

### Фаза 3: Изменение всех виджетов (2 дня)

#### 3.1 Изменить `Button`

**Файл:** `crates/ui/src/widgets/button.rs`

**Было:**
```rust
impl<M: Clone + 'static> Widget<M> for Button<M> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        let available_width = ui.available_width();
        let desired_size = egui::vec2(available_width, self.height);
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
        // ...
    }
}
```

**Стало:**
```rust
impl<M: Clone + 'static> Widget<M> for Button<M> {
    fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<M>) {
        // Measure text
        let galley = ui.painter().layout_no_wrap(
            self.text.clone(),
            egui::FontId::proportional(18.0),
            egui::Color32::WHITE,
        );
        let text_size = galley.size();
        
        // Calculate desired size (wrap-content)
        let btn_width = text_size.x + 24.0; // padding
        let btn_height = self.height.max(text_size.y + 16.0);
        let desired_size = egui::vec2(btn_width, btn_height);
        
        // Allocate respecting constraints
        let (rect, response) = ui.allocate_space_with_sense(
            desired_size,
            egui::Sense::click(),
        );
        
        // Paint
        if ui.is_rect_visible(rect) {
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 4.0, egui::Color32::from_rgb(0, 128, 255));
            let text_pos = egui::pos2(
                rect.center().x - text_size.x / 2.0,
                rect.center().y - text_size.y / 2.0,
            );
            painter.galley(text_pos, galley, egui::Color32::WHITE);
        }
        
        // Handle click
        if response.clicked() {
            if let Some(msg) = &self.on_click_msg {
                dispatch.dispatch(msg.clone());
            }
            if let Some(callback) = &self.on_click_callback {
                callback(ui, dispatch);
            }
        }
    }
}
```

**Зачем:**
- Виджет alloc'ит wrap-content размер
- `allocate_space_with_sense` учитывает constraints
- Если `fill_max_width()` установил `min_width = available_width` — кнопка растянется

#### 3.2 Изменить `Text`

**Файл:** `crates/ui/src/widgets/text.rs`

**Было:**
```rust
impl<M> Widget<M> for Text {
    fn render(&self, ui: &mut egui::Ui, _dispatch: &Dispatcher<M>) {
        // ...
        let (rect, _response) = ui.allocate_exact_size(text_size, egui::Sense::hover());
        // ...
    }
}
```

**Стало:**
```rust
impl<M> Widget<M> for Text {
    fn render(&self, ui: &mut UiWrapper, _dispatch: &Dispatcher<M>) {
        // ...
        let (rect, _response) = ui.allocate_space(text_size);
        // ...
    }
}
```

#### 3.3 Изменить `Spacer`

**Файл:** `crates/ui/src/widgets/spacer.rs`

**Было:**
```rust
impl<M> Widget<M> for Spacer {
    fn render(&self, ui: &mut egui::Ui, _dispatch: &Dispatcher<M>) {
        ui.allocate_exact_size(size, Sense::hover());
    }
}
```

**Стало:**
```rust
impl<M> Widget<M> for Spacer {
    fn render(&self, ui: &mut UiWrapper, _dispatch: &Dispatcher<M>) {
        ui.allocate_space(size);
    }
}
```

#### 3.4 Изменить `Icon`

**Файл:** `crates/ui/src/widgets/icon.rs`

**Было:**
```rust
impl<M> Widget<M> for Icon {
    fn render(&self, ui: &mut egui::Ui, _dispatch: &Dispatcher<M>) {
        ui.add(self.icon.clone());
    }
}
```

**Стало:**
```rust
impl<M> Widget<M> for Icon {
    fn render(&self, ui: &mut UiWrapper, _dispatch: &Dispatcher<M>) {
        ui.add(self.icon.clone());
    }
}
```

**Примечание:** `Icon` использует `ui.add()`, который работает с `egui::Ui` через `Deref`.

---

### Фаза 4: Изменение всех контейнеров (2 дня)

#### 4.1 Изменить `Column`

**Файл:** `crates/ui/src/containers/column.rs`

**Было:**
```rust
impl Column {
    pub fn show<M: 'static, F>(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>, content: F)
    where
        F: FnOnce(&mut egui::Ui, &Dispatcher<M>),
    {
        let render_inner = |ui: &mut egui::Ui| {
            ui.spacing_mut().item_spacing = egui::vec2(self.spacing, self.spacing);
            content(ui, dispatch);
        };
        if self.scrollable {
            egui::ScrollArea::vertical().show(ui, render_inner);
        } else {
            ui.vertical(render_inner);
        }
    }
}
```

**Стало:**
```rust
impl Column {
    pub fn show<M: 'static, F>(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<M>, content: F)
    where
        F: FnOnce(&mut UiWrapper, &Dispatcher<M>),
    {
        let render_inner = |ui: &mut UiWrapper| {
            ui.spacing_mut().item_spacing = egui::vec2(self.spacing, self.spacing);
            content(ui, dispatch);
        };
        
        // Create child UiWrapper with constraints = available_rect
        let available_rect = ui.available_rect_before_wrap();
        let constraints = Constraints::ranged(
            0.0,
            available_rect.width(),
            0.0,
            f32::INFINITY,
        );
        
        let mut child_ui = ui.new_child_with_constraints(
            egui::UiBuilder::new().max_rect(available_rect),
            constraints,
        );
        
        if self.scrollable {
            egui::ScrollArea::vertical().show(&mut child_ui, |ui| {
                render_inner(&mut UiWrapper::new(ui, constraints));
            });
        } else {
            child_ui.vertical(|ui| {
                render_inner(&mut UiWrapper::new(ui, constraints));
            });
        }
    }
}
```

**Зачем:**
- Column создаёт `UiWrapper` с constraints = available_rect
- Дети получают constraints и обязаны их соблюдать
- Если `fill_max_width()` установил `min_width = available_width` — дети растянутся

#### 4.2 Изменить `Row`

**Файл:** `crates/ui/src/containers/row.rs`

**Было:**
```rust
impl Row {
    pub fn new<M: 'static, F>(ui: &mut egui::Ui, dispatch: &Dispatcher<M>, content: F)
    where
        F: FnOnce(&mut egui::Ui, &Dispatcher<M>),
    {
        Self::default().render(ui, dispatch, content);
    }
}
```

**Стало:**
```rust
impl Row {
    pub fn new<M: 'static, F>(ui: &mut UiWrapper, dispatch: &Dispatcher<M>, content: F)
    where
        F: FnOnce(&mut UiWrapper, &Dispatcher<M>),
    {
        Self::default().render(ui, dispatch, content);
    }
    
    fn render<M: 'static, F>(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<M>, content: F)
    where
        F: FnOnce(&mut UiWrapper, &Dispatcher<M>),
    {
        // Create child UiWrapper with constraints
        let available_rect = ui.available_rect_before_wrap();
        let constraints = Constraints::ranged(
            0.0,
            f32::INFINITY, // Row doesn't constrain width
            0.0,
            available_rect.height(),
        );
        
        let mut child_ui = ui.new_child_with_constraints(
            egui::UiBuilder::new().max_rect(available_rect),
            constraints,
        );
        
        child_ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(self.spacing, self.spacing);
            content(&mut UiWrapper::new(ui, constraints), dispatch);
        });
    }
}
```

#### 4.3 Изменить `Stack`

**Файл:** `crates/ui/src/containers/stack.rs`

**Было:**
```rust
impl Stack {
    pub fn new<M: 'static, F>(ui: &mut egui::Ui, dispatch: &Dispatcher<M>, content: F)
    where
        F: FnOnce(&mut egui::Ui, &Dispatcher<M>),
    {
        let available = ui.available_size();
        let (rect, _response) = ui.allocate_exact_size(available, egui::Sense::hover());
        let mut child_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("stack")
                .max_rect(rect)
                .layout(*ui.layout()),
        );
        content(&mut child_ui, dispatch);
    }
}
```

**Стало:**
```rust
impl Stack {
    pub fn new<M: 'static, F>(ui: &mut UiWrapper, dispatch: &Dispatcher<M>, content: F)
    where
        F: FnOnce(&mut UiWrapper, &Dispatcher<M>),
    {
        // Measure all children to get max size
        let available_rect = ui.available_rect_before_wrap();
        
        // Create child UiWrapper with constraints
        let constraints = Constraints::ranged(
            0.0,
            available_rect.width(),
            0.0,
            available_rect.height(),
        );
        
        let mut child_ui = ui.new_child_with_constraints(
            egui::UiBuilder::new()
                .id_salt("stack")
                .max_rect(available_rect),
            constraints,
        );
        
        content(&mut child_ui, dispatch);
        
        // Get max size from children
        let content_size = child_ui.min_size();
        
        // Allocate in parent
        ui.allocate_space(content_size);
    }
}
```

#### 4.4 Изменить `LazyColumn`

**Файл:** `crates/ui/src/containers/lazy_column.rs`

**Было:**
```rust
impl LazyColumn {
    pub fn new<M: 'static, T, F>(
        items: Vec<T>,
        ui: &mut egui::Ui,
        dispatch: &Dispatcher<M>,
        item_builder: F,
    ) where
        F: FnMut(&T, &mut egui::Ui, &Dispatcher<M>),
    {
        Self::default().render(items, ui, dispatch, item_builder);
    }
}
```

**Стало:**
```rust
impl LazyColumn {
    pub fn new<M: 'static, T, F>(
        items: Vec<T>,
        ui: &mut UiWrapper,
        dispatch: &Dispatcher<M>,
        item_builder: F,
    ) where
        F: FnMut(&T, &mut UiWrapper, &Dispatcher<M>),
    {
        Self::default().render(items, ui, dispatch, item_builder);
    }
    
    fn render<M: 'static, T, F>(
        &self,
        items: Vec<T>,
        ui: &mut UiWrapper,
        dispatch: &Dispatcher<M>,
        mut item_builder: F,
    ) where
        F: FnMut(&T, &mut UiWrapper, &Dispatcher<M>),
    {
        let available_rect = ui.available_rect_before_wrap();
        let constraints = Constraints::ranged(
            0.0,
            available_rect.width(),
            0.0,
            f32::INFINITY,
        );
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, self.item_spacing);
            for (index, item) in items.iter().enumerate() {
                ui.push_id(index, |ui| {
                    let mut child_ui = UiWrapper::new(ui, constraints);
                    item_builder(item, &mut child_ui, dispatch);
                });
            }
        });
    }
}
```

---

### Фаза 5: Изменение всех модификаторов (2 дня)

#### 5.1 Изменить `ModifierNode::FillMaxWidth`

**Файл:** `crates/ui/src/modifier/mod.rs`

**Было:**
```rust
ModifierNode::FillMaxWidth => {
    let available = ui.available_size();
    ui.allocate_ui_with_layout(
        egui::vec2(available.x, ui.available_height()),
        *ui.layout(),
        |ui| rest(ui, dispatch),
    );
}
```

**Стало:**
```rust
ModifierNode::FillMaxWidth => {
    let available_width = ui.available_width();
    
    // Create child UiWrapper with min_width = available_width
    let mut new_constraints = *ui.constraints();
    new_constraints.min_width = available_width;
    
    let mut child_ui = ui.new_child_with_constraints(
        egui::UiBuilder::new(),
        new_constraints,
    );
    
    rest(&mut child_ui, dispatch);
    
    // Allocate space in parent
    let content_size = child_ui.min_size();
    ui.allocate_space(content_size);
}
```

**Зачем:**
- `FillMaxWidth` устанавливает `min_width = available_width` в constraints
- Дети получают constraints и обязаны растянуться до `min_width`
- Кнопка alloc'ит `max(desired_size, min_width)` — растягивается

#### 5.2 Изменить `ModifierNode::FillMaxSize`

**Файл:** `crates/ui/src/modifier/mod.rs`

**Было:**
```rust
ModifierNode::FillMaxSize => {
    let available = ui.available_size();
    ui.allocate_ui_with_layout(available, *ui.layout(), |ui| {
        rest(ui, dispatch);
    });
}
```

**Стало:**
```rust
ModifierNode::FillMaxSize => {
    let available_size = ui.available_size();
    
    // Create child UiWrapper with min_width and min_height
    let mut new_constraints = *ui.constraints();
    new_constraints.min_width = available_size.x;
    new_constraints.min_height = available_size.y;
    
    let mut child_ui = ui.new_child_with_constraints(
        egui::UiBuilder::new(),
        new_constraints,
    );
    
    rest(&mut child_ui, dispatch);
    
    // Allocate space in parent
    let content_size = child_ui.min_size();
    ui.allocate_space(content_size);
}
```

#### 5.3 Изменить `ModifierNode::Width` и `Height`

**Файл:** `crates/ui/src/modifier/mod.rs`

**Было:**
```rust
ModifierNode::Width(w) => {
    ui.allocate_ui_with_layout(
        egui::vec2(*w, ui.available_height()),
        *ui.layout(),
        |ui| rest(ui, dispatch),
    );
}
```

**Стало:**
```rust
ModifierNode::Width(w) => {
    // Create child UiWrapper with exact width
    let mut new_constraints = *ui.constraints();
    new_constraints.min_width = *w;
    new_constraints.max_width = *w;
    
    let mut child_ui = ui.new_child_with_constraints(
        egui::UiBuilder::new(),
        new_constraints,
    );
    
    rest(&mut child_ui, dispatch);
    
    // Allocate space in parent
    let content_size = child_ui.min_size();
    ui.allocate_space(content_size);
}
```

#### 5.4 Изменить legacy модификаторы

**Файл:** `crates/ui/src/modifier/legacy.rs`

Все legacy модификаторы (`Padded`, `SizedWidget`, `Background`, `Aligned`, `Clickable`, `ClickableWith`) должны быть изменены аналогично.

**Пример для `SizedWidget`:**

**Было:**
```rust
impl<W: Widget<M>, M> Widget<M> for SizedWidget<W, M> {
    fn render(&self, ui: &mut egui::Ui, dispatch: &Dispatcher<M>) {
        let (rect, _response) = ui.allocate_exact_size(
            egui::vec2(self.width, self.height),
            egui::Sense::hover(),
        );
        let mut child_ui = ui.new_child(
            egui::UiBuilder::new()
                .id_salt("sized_widget")
                .max_rect(rect)
                .layout(*ui.layout()),
        );
        self.inner.render(&mut child_ui, dispatch);
    }
}
```

**Стало:**
```rust
impl<W: Widget<M>, M> Widget<M> for SizedWidget<W, M> {
    fn render(&self, ui: &mut UiWrapper, dispatch: &Dispatcher<M>) {
        // Create child UiWrapper with exact size
        let constraints = Constraints::exact(self.width, self.height);
        
        let mut child_ui = ui.new_child_with_constraints(
            egui::UiBuilder::new().id_salt("sized_widget"),
            constraints,
        );
        
        self.inner.render(&mut child_ui, dispatch);
        
        // Allocate space in parent
        let content_size = child_ui.min_size();
        ui.allocate_space(content_size);
    }
}
```

---

### Фаза 6: Обновление тестов (2 дня)

#### 6.1 Обновить существующие тесты

**Файл:** `crates/ui/tests/widget_tests.rs`

Все тесты должны быть обновлены для работы с `UiWrapper`.

**Пример:**

**Было:**
```rust
#[test]
fn test_button_widget_renders() {
    with_ui(|ui| {
        let btn = Button::<()>::new("Click me");
        btn.render(ui, &Dispatcher::new().0);
    });
}
```

**Стало:**
```rust
#[test]
fn test_button_widget_renders() {
    with_ui(|ui| {
        let mut ui_wrapper = UiWrapper::new(ui, Constraints::default());
        let btn = Button::<()>::new("Click me");
        btn.render(&mut ui_wrapper, &Dispatcher::new().0);
    });
}
```

#### 6.2 Добавить новые тесты на constraints

**Файл:** `crates/ui/tests/constraints_tests.rs`

```rust
#[test]
fn test_fill_max_width_stretches_button() {
    with_ui(|ui| {
        let mut ui_wrapper = UiWrapper::new(ui, Constraints::default());
        let available_width = ui_wrapper.available_width();
        
        Button::<()>::new("Кнопка")
            .modifier(Modifier::new().fill_max_width())
            .render(&mut ui_wrapper, &Dispatcher::new().0);
        
        // Button should stretch to available_width
        // Check that allocated size >= available_width
    });
}

#[test]
fn test_fill_max_width_in_column() {
    with_ui(|ui| {
        let mut ui_wrapper = UiWrapper::new(ui, Constraints::default());
        
        Column::new().show(&mut ui_wrapper, &Dispatcher::new().0, |ui, dispatch| {
            Button::<()>::new("A")
                .modifier(Modifier::new().fill_max_width())
                .render(ui, dispatch);
            Button::<()>::new("B")
                .modifier(Modifier::new().fill_max_width())
                .render(ui, dispatch);
        });
        
        // Both buttons should stretch to column width
        // Check that they don't overlap
    });
}
```

---

### Фаза 7: Добавить новые тесты (1 день)

#### 7.1 Тесты на constraints

- `test_constraints_exact_size`
- `test_constraints_ranged`
- `test_constraints_unconstrained`
- `test_constraints_clamp_size`

#### 7.2 Тесты на UiWrapper

- `test_ui_wrapper_deref`
- `test_ui_wrapper_new_child`
- `test_ui_wrapper_new_child_with_constraints`
- `test_ui_wrapper_allocate_space`

#### 7.3 Тесты на fill_max_width

- `test_fill_max_width_stretches_button`
- `test_fill_max_width_stretches_text`
- `test_fill_max_width_in_column`
- `test_fill_max_width_in_row`
- `test_fill_max_width_in_stack`
- `test_fill_max_width_in_lazy_column`

#### 7.4 Тесты на fill_max_size

- `test_fill_max_size_stretches_button`
- `test_fill_max_size_in_column`

#### 7.5 Тесты на size

- `test_size_exact_width`
- `test_size_exact_height`
- `test_size_in_column`

---

## Итоговый план

| Фаза | Задача | Время |
|------|--------|-------|
| 1 | Создание инфраструктуры | 1 день |
| 2 | Изменение Widget trait | 1 день |
| 3 | Изменение всех виджетов | 2 дня |
| 4 | Изменение всех контейнеров | 2 дня |
| 5 | Изменение всех модификаторов | 2 дня |
| 6 | Обновление тестов | 2 дня |
| 7 | Добавление новых тестов | 1 день |
| **Итого** | | **11 дней** |

**Но можно делать параллельно:**
- Фаза 1 + 2 — последовательно (1 + 1 день)
- Фаза 3 + 4 + 5 — параллельно (2 дня)
- Фаза 6 + 7 — параллельно (2 дня)

**Реально:** ~5-7 дней

---

## Риски и митигации

### Риск 1: Производительность

**Проблема:** Создание `UiWrapper` каждый кадр может быть медленным.

**Митигация:**
- `UiWrapper` — это тонкая обёртка, не копирует данные
- Только ссылка на `egui::Ui` + `Constraints` (8 байт)
- overhead минимальный

### Риск 2: Сложность рефакторинга

**Проблема:** Нужно изменить много файлов.

**Митигация:**
- Alpha версия — большой рефакторинг не страшен
- Можно делать постепенно (фаза за фазой)
- Каждый коммит — рабочая версия

### Риск 3: Обратная совместимость

**Проблема:** Ломаем API.

**Митигация:**
- Alpha версия — обратная совместимость не важна
- Пользователи знают, что API может меняться

### Риск 4: Циклическая зависимость

**Проблема:** `core` зависит от `ui`, `ui` зависит от `core`.

**Митигация:**
- Переместить `UiWrapper` и `Constraints` в `core`
- `core` — базовый крейт, все от него зависят

---

## Масштабируемость

### Почему это масштабируемо?

1. **Расширяемость `Constraints`**
   - Можно добавлять новые поля (например, `aspect_ratio`, `min_aspect_ratio`)
   - Не нужно менять виджеты
   
2. **Расширяемость `UiWrapper`**
   - Можно добавлять новые методы
   - Не нужно менять egui
   
3. **Тестируемость**
   - Можно мокать `UiWrapper` в тестах
   - Не нужно создавать реальный `egui::Ui`
   
4. **Независимость от egui**
   - Если перейдём на другой GUI — обёртка останется
   - Меняется только внутренняя реализация
   
5. **Чистая архитектура**
   - Виджеты не знают про контейнер, только про constraints
   - Родитель управляет layout через constraints

### Примеры расширения

#### Добавление `aspect_ratio`

```rust
pub struct Constraints {
    pub min_width: f32,
    pub max_width: f32,
    pub min_height: f32,
    pub max_height: f32,
    pub aspect_ratio: Option<f32>, // новое поле
}
```

Виджеты не нужно менять — они просто игнорируют `aspect_ratio`, если не поддерживают.

#### Добавление `min_aspect_ratio`

```rust
pub struct Constraints {
    // ...
    pub min_aspect_ratio: Option<f32>,
    pub max_aspect_ratio: Option<f32>,
}
```

Виджеты могут использовать для расчёта размера.

#### Добавление нового модификатора

```rust
pub enum ModifierNode<M> {
    // ...
    AspectRatio(f32),
}
```

Модификатор устанавливает `constraints.aspect_ratio = Some(ratio)`.

---

## Итог

**Что меняем:**
1. Создаём `Constraints` и `UiWrapper`
2. Изменяем `Widget` trait
3. Изменяем все виджеты (Button, Text, Spacer, Icon)
4. Изменяем все контейнеры (Column, Row, Stack, LazyColumn)
5. Изменяем все модификаторы (legacy + value type)
6. Обновляем тесты
7. Добавляем новые тесты

**Зачем:**
- Чистая архитектура (виджеты не знают про контейнер)
- Compose-like Constraints
- Масштабируемость
- Тестируемость
- Независимость от egui

**Почему такая реализация:**
- `Deref<Target = egui::Ui>` — совместимость с существующим кодом
- `Constraints` как отдельная struct — расширяемость
- Перемещение в `core` — избегаем циклической зависимости

**Масштабируемость:**
- Можно добавлять новые поля в `Constraints`
- Можно добавлять новые методы в `UiWrapper`
- Можно добавлять новые модификаторы
- Можно перейти на другой GUI

**Время:** ~5-7 дней (параллельно)

**Риски:**
- Производительность — минимальный overhead
- Сложность — alpha версия, не страшно
- Обратная совместимость — не важна
- Циклическая зависимость — решено перемещением в `core`

---

## Следующие шаги

1. **Создать PR с планом миграции** (этот документ)
2. **Обсудить план с командой**
3. **Начать с Фазы 1** (создание инфраструктуры)
4. **Делать фазы последовательно** (каждая фаза — рабочий коммит)
5. **Тестировать после каждой фазы**
6. **Обновлять документацию**

---

**Готов начать?**
