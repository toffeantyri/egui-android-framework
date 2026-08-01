# egui-android-navigation

**Навигация с управлением жизненным циклом для egui-приложений на Android.**

`ChildStack` — стек компонентов с управлением жизненным циклом,
аналог `ChildStack` из Decompose. Поддерживает push/pop/replace,
обработку кнопки Back, save/restore состояния и фабрику компонентов.

[![crates.io](https://img.shields.io/crates/v/egui-android-navigation)](https://crates.io/crates/egui-android-navigation)

## Проблема

В многоэкранном приложении нужно:
- Переключаться между экранами с историей переходов (назад)
- Управлять жизненным циклом: создавать экран при входе, уничтожать при выходе
- Обрабатывать системную кнопку Back (кастомная логика → pop стека → выход)
- Сохранять и восстанавливать стек навигации при повороте экрана

`ChildStack` решает все эти задачи.

## Возможности

- **`push(config, component)`** — добавить компонент на стек с lifecycle (on_create → on_start → on_resume)
- **`pop()`** — удалить с корректным lifecycle (on_pause → on_stop → on_destroy). Возвращает `Option<(C, Box<dyn ComponentNode>)>`
- **`replace(config, component)`** — заменить верхний компонент (pop + push за один lifecycle-цикл)
- **`bring_to_front(config, component)`** — добавить на вершину стека, если такого ещё нет
- **`clear()`** — очистить стек с destroy всех компонентов
- **`on_back(ctx) -> BackAction`** — Decompose-style обработка Back. `active.handle_back(ctx)` возвращает `BackAction`: `Handled`/`Pop`/`Finish`/`Propagate`. Если `Propagate` — `pop()` если стек > 1, иначе `Finish`
- **`active() / active_mut()`** — доступ к активному компоненту
- **`save() -> SavedStack<C>`** — сериализация стека для save/restore
- **`restore(saved, factory)`** — восстановление стека из сохранённого состояния
- **`restore_from_saved(saved, factory)`** — восстановление с пересозданием компонентов через фабрику
- **`is_empty() / len()`** — состояние стека

### `ComponentFactory<C>` (trait)
- Фабрика компонентов по конфигурации (маршруту)
- Позволяет отделить создание компонентов от навигации (OCP)
- Аналог `Router` в Decompose

```rust
impl ComponentFactory<Route> for MyFactory {
    fn create(&self, config: Route) -> Box<dyn ComponentNode> {
        match config {
            Route::Home => Box::new(HomeScreen::new()),
            Route::Settings => Box::new(SettingsScreen::new()),
        }
    }
}
```

### `LifecycleEvent` (enum)
- `Resume / Pause` — события жизненного цикла для проброса в стек

## Пример

```rust
use egui_android_navigation::{ChildStack, ComponentFactory};
use egui_android_core::{ComponentContext, ComponentNode};

enum Route { Home, Settings }

struct MyFactory;
impl ComponentFactory<Route> for MyFactory {
    fn create(&self, config: Route) -> Box<dyn ComponentNode> {
        match config {
            Route::Home => Box::new(HomeScreen::new()),
            Route::Settings => Box::new(SettingsScreen::new()),
        }
    }
}

let factory = MyFactory;
let mut stack = ChildStack::new();
stack.push(Route::Home, factory.create(Route::Home));
stack.push(Route::Settings, factory.create(Route::Settings));

let mut ctx = ComponentContext::new();
if stack.on_back(&mut ctx) {
    // Settings popped, вернулись на Home
}

// Save/restore при повороте экрана
let saved = stack.save();
stack.restore_from_saved(saved, &factory);
```

## Когда использовать

Подключайте `egui-android-navigation`, если вам нужна навигация с историей переходов,
жизненным циклом экранов и save/restore.

Для обычного использования все типы доступны через [`egui-android-framework`](https://crates.io/crates/egui-android-framework):

```rust
use egui_android::navigation::{ChildStack, ComponentFactory};
```

## Зависимости

Зависит от: [`egui-android-core`](https://crates.io/crates/egui-android-core), [`egui-android-runtime`](https://crates.io/crates/egui-android-runtime), [`egui-android-ui`](https://crates.io/crates/egui-android-ui), `serde`, `bincode`
