План: Единая система обработки Back (Decompose-style)

### Проблема

Сейчас есть две независимые системы, которые не связаны друг с другом:

| Точка входа | Путь | Кто обрабатывает |
|---|---|---|
| **Системный Back** (жест/кнопка) | `platform-android → app.on_back_pressed() → NavigationHost::on_back() → ChildStack::on_back() → active.handle_back()` | `ComponentNode::handle_back()` |
| **Кнопка "← Назад" в UI** | `dispatch(RootMsg::Back) → NavigationHost::handle_msg(RootMsg::Back) → NavigationHost::on_back()` | `NavigationHost::handle_msg()` |

**Корнер-кейсы, которые не работают:**
1. Экран с не-RootMsg (StateScreen) не может иметь кнопку "← Назад" — тип не позволяет диспатчить `RootMsg::Back`
2. "Сделать что-то + pop" (сброс счётчика + навигация) — два действия, которые нельзя совместить
3. Кастомная логика при Back (BackCustomScreen) работает, но только через системный Back — кнопка "← Назад" в UI не вызывает кастомную логику

---

### Схема: новая единая система

```
                    ┌──────────────────────────────────────┐
                    │          BackPressedProcessor          │
                    │  (единственная точка входа)            │
                    └──────┬───────────────────────────────┘
                           │
            ┌──────────────┼──────────────┐
            ▼              ▼              ▼
    ┌──────────────┐ ┌──────────┐ ┌──────────────┐
    │ Системный    │ │ Кнопка   │ │ Кастомная    │
    │ Back (андр.) │ │ "← Назад"│ │ логика       │
    │              │ │ в UI     │ │ (on_click_with)│
    └──────┬───────┘ └────┬─────┘ └──────┬───────┘
           │              │              │
           └──────────────┼──────────────┘
                          ▼
           ┌──────────────────────────────┐
           │  ChildStack::on_back()       │
           │                              │
           │  1. active.handle_back()     │ ← кастомный перехват (Nested)
           │                              │
           │  2. BackDispatcher::handle() │ ← BackHandler-ы (диалоги)
           │                              │
           │  3. active.take_back_req()   │ ← "сброс + pop" (флаг)
           │                              │
           │  4. pop()                    │ ← если стек > 1
           │                              │
           │  5. finish_requested = true  │ ← если стек = 1 (Home)
           └──────────────────────────────┘
```

### Изменения

#### 1. `ComponentNode` — добавить `take_back_request()`

```rust
pub trait ComponentNode: LifecycleObserver + Send + 'static {
    // ... существующие методы ...

    /// Проверить, запросил ли компонент навигацию назад.
    /// Вызывается после render + handle_dyn.
    /// Если true — ChildStack сделает pop (или finish_requested).
    ///
    /// Используется для сценария "сделать что-то + pop":
    /// handle(MyMsg::BackAndReset) → сброс + back_requested = true
    fn take_back_request(&mut self) -> bool {
        false
    }
}
```

#### 2. `Macros` — обновить `#[derive(ComponentNode)]`

Для компонентов с `#[persistent_fields]`:
```rust
// Генерируется макросом:
impl ComponentNode for StateScreen {
    fn take_back_request(&mut self) -> bool {
        std::mem::replace(&mut self.__back_requested, false)
    }
    
    fn handle_dyn(&mut self, msg: Box<dyn Any + Send>) {
        if let Ok(typed) = msg.downcast::<StateScreenMsg>() {
            if matches!(typed, StateScreenMsg::Back) {
                self.__back_requested = true;
            }
            Component::handle(self, typed);
        }
    }
}
```

**Но это сложно и ломает макрос.** Лучше: добавить общий метод `take_back_request` на `ComponentNode`, который по умолчанию возвращает `false`. Компонент сам выставляет флаг:

```rust
// StateScreen
fn handle(&mut self, msg: Self::Message) {
    match msg {
        StateScreenMsg::Back => {
            self.counter = 0;
            // Signal navigation host to go back
        }
    }
}
```

Вопрос: как сигнализировать? `ComponentNode` — object-safe, нельзя вернуть `Result` из `handle_dyn`. 

**Решение:** `NavigationHost` после `handle_dyn` **всегда проверяет**, нужно ли делать pop. Механизм: `ChildStack::on_back()` уже делает эту цепочку. Просто вызывать `on_back()` после `handle_msg()`.

Сейчас `handle_msg(RootMsg::Back)` вызывает `self.on_back()`. А `handle_dyn` для не-RootMsg **не вызывает** `on_back()` — и это правильно, потому что `StateScreenMsg::Reset` не должен делать pop.

**Для сценария "сброс + pop":** Новый метод `handle_msg` должен уметь сказать: "после обработки сообщения сделай pop". Самый простой путь — `handle_msg` возвращает `bool` (нужен ли pop):

```rust
// NavigationHost
fn handle_msg(&mut self, msg: RootMsg) {
    match msg {
        RootMsg::Back => self.on_back(),
        // ...
    }
}

// После dispatch, в app.rs::frame():
for msg in uidynmsg_rx.try_iter() {
    match msg.downcast::<RootMsg>() {
        Ok(root_msg) => self.root.handle_msg(*root_msg),
        Err(msg) => {
            if let Some(active) = self.root.stack.active_mut() {
                active.handle_dyn(msg);
            }
        }
    }
}
```

**Но как после `handle_dyn` понять, нужно ли делать pop?** `handle_dyn` не возвращает значение. А добавлять `-> bool` — ломать object-safe интерфейс.

#### 3. `NavigationHost` — проверять `take_back_request()` после обработки сообщений

```rust
// app.rs::frame():
for msg in uidynmsg_rx.try_iter() {
    match msg.downcast::<RootMsg>() {
        Ok(root_msg) => self.root.handle_msg(*root_msg),
        Err(msg) => {
            if let Some(active) = self.root.stack.active_mut() {
                active.handle_dyn(msg);
                // После handle_dyn проверяем, запросил ли компонент Back
                if active.take_back_request() {
                    self.root.on_back();
                }
            }
        }
    }
}
```

Компонент в `handle(MyMsg::Back)` выставляет `self.back_requested = true`. `ComponentNode::take_back_request()` читает и сбрасывает флаг. NavigationHost проверяет после `handle_dyn` и делает pop.

### Что нужно сделать (файлы)

| # | Файл | Изменение |
|---|---|---|
| 1 | `crates/core/src/component_node.rs` | Добавить `fn take_back_request(&mut self) -> bool { false }` |
| 2 | `crates/macros/src/lib.rs` | Не менять — `take_back_request` имеет реализацию по умолчанию. Компонент сам переопределяет, если нужно |
| 3 | `examples/showcase/src/navigation_host.rs` | Добавить `check_back_request()` — метод, который проверяет флаг у активного компонента и вызывает `on_back()` |
| 4 | `examples/showcase/src/app.rs` | После `handle_dyn` вызывать `self.root.check_back_request()` |
| 5 | `examples/showcase/src/screens/state_screen.rs` | Добавить поле `back_requested: bool`. В `handle(StateScreenMsg::Back)` — сброс + `back_requested = true`. Кнопка "← Назад" → `StateScreenMsg::Back` |

### Корнер-кейсы

| Сценарий | Как работает |
|---|---|
| **Системный Back** (Android, любой экран) | `process_back_pressed()` → `app.on_back_pressed()` → `NavigationHost::on_back()` → `ChildStack::on_back()` → `active.handle_back()` → pop/finish |
| **Кнопка "← Назад" (RootMsg::Back)** | `dispatch(RootMsg::Back)` → downcast в NavigationHost.handle_msg → `self.on_back()` → `ChildStack::on_back()` |
| **Кнопка "← Назад" (StateScreenMsg::Back)** | `dispatch(StateScreenMsg::Back)` → downcast НЕ RootMsg → `handle_dyn` → `handle(StateScreenMsg::Back)` → сброс + `back_requested = true` → после frame `check_back_request()` → `on_back()` → pop |
| **Кастомный перехват** (NestedScreen, BackCustomScreen) | `handle_back()` → true → Back обработан, pop не делается. Кнопка "← Назад" в UI не нужна — системный Back делает то же самое |
| **Диалоги** (будущее) | `BackDispatcher::register(BackHandler{priority: 100, ...})` в `render()` → `ChildStack::on_back()` вызывает `BackDispatcher::handle()` перед pop |
| **Сброс + pop** (StateScreen) | Кнопка "← Назад" → `StateScreenMsg::Back` → `handle(Back)` → сброс + флаг → `take_back_request()` → pop |
| **Сброс без pop** (кнопка Reset) | `StateScreenMsg::Reset` → `handle(Reset)` → только сброс, без флага → pop не делается |
| **HomeScreen (корень)** | `ChildStack::on_back()` → стек = 1 → `finish_requested = true` → приложение завершается
