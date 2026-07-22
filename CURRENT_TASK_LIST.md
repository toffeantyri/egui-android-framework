--- Задача ---
Архитектура Decompose и наше соответствие

### Ключевые сущности Decompose → Наш аналог

| Decompose | Наш аналог | Статус |
|---|---|---|
| `Parcelable` конфигурация | `C: Clone + Debug + Serialize + Deserialize` | Нужно добавить serde |
| `ChildStack<C, T>` | `ChildStack<C>` (хранит `Box<dyn ComponentNode>`) | ✅ Есть |
| `ComponentContext` | `ComponentContext<NavEvent, DataCmd, State>` | ✅ Есть |
| `StateKeeper` (регистрируется в контексте) | `ComponentNode::save_state()` + `restore_state()` | ✅ Есть |
| `StateKeeperDispatcher` | `ChildStack::save()` / `restore()` — рекурсивный обход | ✅ Есть, но не подключён |
| `SavedState` (Parcelable контейнер) | `Vec<u8>` через bincode | Нужно создать |
| `GenericComponentContext` (корень) | `NavigationHost` (корневой компонент) | ✅ Есть |
| `onSaveInstanceState(Bundle)` | `Application::on_save_state() -> SavedState` | Нужно реализовать |
| `onCreate(Bundle?)` | `Application::on_restore_state(SavedState)` | Нужно реализовать |
| `RootComponent` | `NavigationHost` (не реализует Component) | ✅ Есть |

---

### Как работает Decompose при Save

```
Activity.onSaveInstanceState(outState)
  ↓
ComponentContext.saveState(outState)
  ↓
StateKeeperDispatcher.save()
  │
  ├── Для Child Stack:
  │   1. Сохранить конфигурации всех элементов стека
  │   2. Для каждого элемента → Component.saveState()
  │      └── Внутри компонента: все зарегистрированные StateKeeper-ы
  │         сохраняют свои данные
  │
  └── Все данные → SavedState (Parcelable) → outState
```

### Как работает Decompose при Restore

```
Activity.onCreate(savedInstanceState)
  ↓
savedInstanceState?.getParcelable<SavedState>()
  ↓
ComponentContext.restoreState(savedState)
  ↓
StateKeeperDispatcher.restore(data)
  │
  ├── Для Child Stack:
  │   1. Извлечь сохранённые конфигурации
  │   2. Пересоздать компоненты заново (factory.create(config))
  │   3. Для каждого → Component.restoreState()
  │      └── Внутри: StateKeeper-ы восстанавливают данные
```

**Критически важно:** при restore компоненты **пересоздаются**, не "чинятся". Конфигурация — единственный источник истины для структуры стека.

---

### Три слоя сохранения (как в Decompose)

```
Слой 1: Android Bundle
  └── Parcelable byte array → Vec<u8>
      │
      Слой 2: StateKeeperDispatcher (ChildStack::save/restore)
      │
      ├── Стек: список конфигураций
      └── Состояния компонентов: Vec<u8> для каждого
          │
          Слой 3: Компоненты (ComponentNode::save_state/restore_state)
          │
          ├── Примитивные данные (счётчики, текст)
          ├── Вложенные ChildStack (рекурсивно!)
          └── Any кастомные данные
```

---

## Задача: полная реализация

### Что должно быть на выходе

1. **Поворот экрана:** пользователь на экране State (счётчик = 5) → поворот → счётчик всё ещё 5, экран тот же
2. **Вложенная навигация:** пользователь на Nested → Экран B → поворот → Nested → Экран B
3. **Kill/Restore процесса:** система убила процесс → перезапуск → открывается тот же стек с теми же данными
4. **Произвольные данные:** любой компонент может сохранить любые `Serialize + Deserialize` данные

---

## Пошаговый план реализации

### Шаг 0. Добавить зависимости

**Крейт: `egui-android-runtime`**
- В `Cargo.toml` добавить `serde` с derive, `bincode`

**Крейт: `egui-android-navigation`**
- В `Cargo.toml` добавить `serde`, `bincode`

**Крейт: `egui-android-core`**
- В `Cargo.toml` добавить `serde`

---

### Шаг 1. Тип `SavedState` в `egui-android-runtime`

**Файл: `crates/runtime/src/saved_state.rs`** (новый)

```rust
/// Сохранённое состояние приложения.
/// Это аналог `SavedState` из Decompose — контейнер Parcelable-данных.
/// Сериализуется через bincode и сохраняется в Android Bundle как byte array.
pub type SavedState = Option<Vec<u8>>;
```

**Файл: `crates/runtime/src/lib.rs`** — добавить модуль, реэкспорт типа.

---

### Шаг 2. Расширить `Application` trait

**Файл: `crates/runtime/src/application.rs`**

```rust
pub trait Application {
    // ... существующие методы ...

    /// Сохранить состояние навигации и компонентов.
    /// Возвращает сериализованные данные для Android Bundle.
    /// 
    /// Вызывается из platform-android при Lifecycle::Destroy.
    fn on_save_state(&mut self) -> SavedState { None }

    /// Восстановить состояние навигации и компонентов.
    /// 
    /// Вызывается из platform-android при первом InitWindow.
    fn on_restore_state(&mut self, _state: SavedState) {}
}
```

---

### Шаг 3. Platform-android: хранить и пробрасывать SavedState

**Файл: `crates/platform-android/src/loop.rs`** — `RunState`

Добавить поле:
```rust
pub struct RunState {
    // ... существующие поля ...
    /// Сохранённое состояние навигации.
    /// Сохраняется при Destroy, передаётся в on_restore_state при InitWindow.
    pub saved_state: Option<Vec<u8>>,
}
```

**Файл: `crates/platform-android/src/lifecycle.rs`**

Изменить `handle_destroy`:
```rust
fn handle_destroy<A: Application>(
    app_instance: &mut A,
    destroy_requested: &mut bool,
) -> SavedState {
    let saved = app_instance.on_save_state();
    *destroy_requested = true;
    saved
}
```

Изменить `handle_init_window`:
```rust
fn handle_init_window<A: Application>(
    backend: &mut dyn AndroidBackend,
    app_instance: &mut A,
    egui_ctx: &egui::Context,
    graphics: &mut Option<GraphicsPipeline>,
    saved_state: &mut Option<Vec<u8>>,
) {
    if !has_egl {
        // Первый InitWindow
        backend.init().ok();
        backend.init_graphics().ok();
        
        // Восстанавливаем сохранённое состояние
        let state = saved_state.take();
        app_instance.on_restore_state(state);
    } else {
        backend.recreate_surface().ok();
    }
    // ...
}
```

Изменить `handle_lifecycle_event` — добавить параметр `saved_state` и возвращать `SavedState` из Destroy.

**Файл: `crates/platform-android/src/loop.rs`** — `tick()`

```rust
// При Destroy:
BackendEvent::Lifecycle(ev) => {
    let saved = crate::lifecycle::handle_lifecycle_event(
        ev, backend, app_instance, egui_ctx,
        &mut self.graphics, &mut self.destroy_requested,
        &mut self.saved_state,
    );
    if let Some(bytes) = saved {
        self.saved_state = Some(bytes);
    }
}
```

---

### Шаг 4. Сделать `C` (конфигурацию) сериализуемой в `ChildStack`

**Файл: `crates/navigation/src/child_stack.rs`**

Изменить bound на `C`:
```rust
// Было:
C: Clone + PartialEq + std::fmt::Debug,

// Стало:
C: Clone + PartialEq + std::fmt::Debug + Serialize + DeserializeOwned,
```

Добавить метод `save_serializable()`:
```rust
/// Сохранить стек с сериализованными состояниями компонентов.
/// Использует bincode для сериализации ComponentNode::save_state().
pub fn save_serializable(&self) -> Vec<(C, Option<Vec<u8>>)> {
    self.items
        .iter()
        .map(|item| {
            let state_bytes = item.component.save_state()
                .and_then(|state| {
                    // Пробуем сериализовать через bincode
                    // Но save_state() возвращает Box<dyn Any + Send>,
                    // а не Serialize. Нужен другой подход — см. Шаг 7.
                    None
                });
            (item.config.clone(), state_bytes)
        })
        .collect()
}
```

**Проблема:** `save_state()` возвращает `Box<dyn Any + Send>`, который нельзя сериализовать через bincode. Нужно либо:
- Изменить сигнатуру на `Box<dyn SerializableState>` (новый trait)
- Или сериализовать компонентом отдельно через trait `PersistentState`

**Решение (как в Decompose):** Ввести трейт `PersistentState` + метод `save_to_bytes()`.

---

### Шаг 5. Трейт `PersistentState` в `egui-android-core`

**Файл: `crates/core/src/persistent_state.rs`** (новый)

```rust
use serde::{Serialize, Deserialize};

/// Трейт для типобезопасного сохранения/восстановления состояния.
/// Аналог `StateKeeper` в Decompose.
///
/// Компонент реализует этот трейт, если хочет сохранять кастомные данные
/// при пересоздании Activity (поворот экрана, kill/restore).
pub trait PersistentState {
    /// Тип сохраняемого состояния.
    /// Должен быть Serializable + Deserializable + Send.
    type State: Serialize + DeserializeOwned + Send + 'static;

    /// Сохранить текущее состояние.
    fn save(&self) -> Self::State;

    /// Восстановить состояние из ранее сохранённого.
    fn restore(&mut self, state: Self::State);
}
```

---

### Шаг 6. Связать `ComponentNode::save_state()` с `PersistentState`

**Вариант A (blanket-impl):** если компонент реализует `PersistentState`, то `save_state()`/`restore_state()` автоматически сериализуют через bincode.

```rust
impl<T: PersistentState + ComponentNode> ComponentNodeExt for T {
    fn save_state(&self) -> Option<Box<dyn Any + Send>> {
        let data = self.save();
        let bytes = bincode::serialize(&data).ok()?;
        Some(Box::new(bytes))
    }
}
```

**Вариант Б (явный в `ComponentNode`):** добавить методы `save_to_bytes()` / `restore_from_bytes()`:

```rust
pub trait ComponentNode {
    // ... существующие методы ...

    /// Сохранить состояние компонента как сериализованные байты.
    fn save_state(&self) -> Option<Box<dyn Any + Send>> { None }
    
    // Существующий restore_state остаётся
}
```

И в `ChildStack::save()` сериализовать через bincode если тип реализует `Serialize`.

**Решение:** идём по пути **Варианта А** — вводим `PersistentState` как отдельный трейт и делаем blanket-impl для `ComponentNode`, который автоматически конвертирует `Serialize` данные в `Vec<u8>`.

---

### Шаг 7. `ChildStack` — сериализуемое сохранение/восстановление

**Файл: `crates/navigation/src/child_stack.rs`**

Добавить struct для сохранённых данных:

```rust
/// Сохранённое представление стека.
#[derive(Serialize, Deserialize)]
pub struct SavedStack<C> {
    /// Элементы стека: конфигурация + сериализованное состояние компонента.
    pub items: Vec<(C, Option<Vec<u8>>)>,
}
```

Изменить `save()` — возвращать `SavedStack<C>`:
```rust
pub fn save(&self) -> SavedStack<C> {
    let items = self.items.iter().map(|item| {
        let state_bytes = item.component.save_state()
            .and_then(|boxed| {
                // downcast to Vec<u8> (если компонент реализует PersistentState)
                boxed.downcast::<Vec<u8>>().ok().map(|v| *v)
            });
        (item.config.clone(), state_bytes)
    }).collect();
    SavedStack { items }
}
```

Переделать `restore()` — принимать `SavedStack<C>`:
```rust
/// Восстановить стек из сохранённого состояния.
/// Компоненты пересоздаются через фабрику, затем восстанавливают состояние.
/// Аналог Decompose: пересоздание компонентов + restoreState.
pub fn restore_from_saved(
    &mut self,
    saved: SavedStack<C>,
    factory: &dyn ComponentFactory<C>,
) {
    self.clear();
    for (config, state_bytes) in saved.items {
        let mut component = factory.create(config.clone());
        if let Some(bytes) = state_bytes {
            component.restore_state(Box::new(bytes));
        }
        self.push(config, component);
    }
}
```

---

### Шаг 8. `NavigationHost` — save/restore (showcase)

**Файл: `examples/showcase/src/navigation_host.rs`**

Добавить методы:

```rust
use egui_android_runtime::saved_state::SavedStack;

impl NavigationHost {
    /// Сохранить состояние всей навигации.
    pub fn save(&self) -> SavedStack<Route> {
        self.stack.save()
    }

    /// Восстановить навигацию из сохранённого состояния.
    pub fn restore(&mut self, saved: SavedStack<Route>) {
        self.stack.restore_from_saved(saved, &*self.factory);
    }
}
```

---

### Шаг 9. `ShowcaseApplication` — подключить save/restore

**Файл: `examples/showcase/src/app.rs`**

```rust
use egui_android_runtime::saved_state::{SavedState, SavedStack};

impl Application for ShowcaseApplication {
    // ... существующие методы ...

    fn on_save_state(&mut self) -> SavedState {
        let saved = self.root.save();
        let bytes = bincode::serialize(&saved)
            .expect("Ошибка сериализации SavedStack");
        log::info!("on_save_state: сохранено {} элементов стека", saved.items.len());
        Some(bytes)
    }

    fn on_restore_state(&mut self, state: SavedState) {
        if let Some(bytes) = state {
            match bincode::deserialize::<SavedStack<Route>>(&bytes) {
                Ok(saved) => {
                    log::info!("on_restore_state: восстановлено {} элементов", saved.items.len());
                    self.root.restore(saved);
                }
                Err(e) => {
                    log::error!("on_restore_state: ошибка десериализации: {}", e);
                }
            }
        }
    }
}
```

---

### Шаг 10. Рекурсивное сохранение вложенных стеков

**Файл: `examples/showcase/src/screens/nested.rs`**

`NestedScreen` должен реализовать `PersistentState` (или переопределить `save_state`/`restore_state`), чтобы сохранять свои `stack_layer1`, `stack_layer2` и `layer2_open`.

```rust
// Сохраняемая структура
#[derive(Serialize, Deserialize)]
struct NestedSavedState {
    layer1: SavedStack<NestedRoute>,
    layer2: SavedStack<NestedLayer2Route>,
    layer2_open: bool,
}

impl PersistentState for NestedScreen {
    type State = NestedSavedState;

    fn save(&self) -> Self::State {
        NestedSavedState {
            layer1: self.stack_layer1.save(),
            layer2: self.stack_layer2.save(),
            layer2_open: self.layer2_open,
        }
    }

    fn restore(&mut self, state: Self::State) {
        // ВАЖНО: пересоздаём компоненты через фабрики
        struct Layer1Factory;
        impl ComponentFactory<NestedRoute> for Layer1Factory {
            fn create(&self, config: NestedRoute) -> Box<dyn ComponentNode> {
                Box::new(Layer1Sub::from_route(&config))
            }
        }
        struct Layer2Factory;
        impl ComponentFactory<NestedLayer2Route> for Layer2Factory {
            fn create(&self, config: NestedLayer2Route) -> Box<dyn ComponentNode> {
                Box::new(Layer2Sub::from_route(&config))
            }
        }

        // Очищаем и пересоздаём
        self.stack_layer1.clear();
        self.stack_layer2.clear();

        // Восстанавливаем слой 1
        let mut new_stack1 = ChildStack::new();
        new_stack1.restore_from_saved(state.layer1, &Layer1Factory);
        self.stack_layer1 = new_stack1;

        // Восстанавливаем слой 2
        let mut new_stack2 = ChildStack::new();
        new_stack2.restore_from_saved(state.layer2, &Layer2Factory);
        self.stack_layer2 = new_stack2;

        self.layer2_open = state.layer2_open;
    }
}
```

---

### Шаг 11. Пример кастомных данных компонента (StateScreen)

**Файл: `examples/showcase/src/screens/state_screen.rs`**

```rust
#[derive(Serialize, Deserialize)]
struct StateScreenSavedState {
    counter: i32,
    expanded: bool,
}

// StateScreen получает поля для хранения между пересозданиями
pub struct StateScreen {
    counter: i32,
    expanded: bool,
}

impl PersistentState for StateScreen {
    type State = StateScreenSavedState;

    fn save(&self) -> Self::State {
        StateScreenSavedState {
            counter: self.counter,
            expanded: self.expanded,
        }
    }

    fn restore(&mut self, state: Self::State) {
        self.counter = state.counter;
        self.expanded = state.expanded;
    }
}
```

---

### Шаг 12. Тесты

**Файл: `crates/navigation/src/child_stack.rs`** — добавить тесты:

```rust
#[test]
fn test_save_restore_stack() {
    let mut stack = ChildStack::<TestRoute>::new();
    stack.push(TestRoute::A, Box::new(TestComp::new(42)));
    stack.push(TestRoute::B, Box::new(TestComp::new(99)));

    let saved = stack.save();
    assert_eq!(saved.items.len(), 2);
    assert_eq!(saved.items[0].0, TestRoute::A);
    assert_eq!(saved.items[1].0, TestRoute::B);

    // Пересоздаём стек из сохранённого
    let mut restored = ChildStack::new();
    restored.restore_from_saved(saved, &TestFactory);
    assert_eq!(restored.len(), 2);

    // Проверяем, что состояние компонентов восстановилось
    // (нужен TestComp с PersistentState и сохранённым значением)
}
```

**Файл: `crates/runtime/src/saved_state.rs`** — тесты сериализации/десериализации `SavedStack`.

---

### Шаг 13. Документация

Обновить:
- `SKILL.md` в `egui-android-guide` — добавить раздел "Сохранение состояния"
- `SKILL.md` в `android-egui-architecture` — добавить слой "SavedStateRegistry"
- Комментарии в коде — на русском языке

---

## Результирующая файловая структура (новые/изменённые файлы)

```
crates/
├── runtime/src/
│   ├── saved_state.rs       ← НОВЫЙ: SavedState тип, SavedStack, утилиты
│   ├── application.rs       ← ИЗМЕНИТЬ: on_save_state/on_restore_state с SavedState
│   └── lib.rs               ← ИЗМЕНИТЬ: pub mod saved_state
│
├── core/src/
│   ├── persistent_state.rs  ← НОВЫЙ: трейт PersistentState
│   ├── component_node.rs    ← ИЗМЕНИТЬ: blanket-impl для PersistentState
│   └── lib.rs               ← ИЗМЕНИТЬ: pub mod persistent_state
│
├── navigation/src/
│   ├── child_stack.rs       ← ИЗМЕНИТЬ: SavedStack, save/restore через bincode
│   └── lib.rs               ← ИЗМЕНИТЬ: pub use SavedStack
│
├── platform-android/src/
│   ├── loop.rs              ← ИЗМЕНИТЬ: RunState.saved_state
│   ├── lifecycle.rs         ← ИЗМЕНИТЬ: проброс saved_state
│   └── run.rs               ← ИЗМЕНИТЬ: передача saved_state в lifecycle
│
└── framework/src/
    └── lib.rs               ← ИЗМЕНИТЬ: re-export нового API
```

---

## Итоговая проверка: все правила Decompose соблюдены

| Принцип Decompose | Как реализовано |
|---|---|
| Конфигурация = источник истины | `C: Serialize + Deserialize` используется для пересоздания компонентов |
| Компоненты пересоздаются при restore | `ComponentFactory::create(config)` + затем `restore_state()` |
| Рекурсивное сохранение | `NestedScreen` сохраняет свои `ChildStack` через `PersistentState` |
| Единый контейнер (SavedState) | `SavedStack<C>` — одно значение, сериализуемое в Bundle |
| Android Bundle как транспорт | `Vec<u8>` → `Bundle.putByteArray()` |
| StateKeeper регистрируется/отписывается | `ComponentNode::save_state()` вызывается только для живых компонентов в стеке |

---

## Порядок выполнения

**Шаги идут строго последовательно из-за зависимостей:**

```
0. dep: serde, bincode в runtime, core, navigation
   ↓
1. saved_state.rs (новый модуль в runtime)
   ↓
2. application.rs (расширить trait)
   ↓
3. loop.rs + lifecycle.rs (platform-android)
   ↓
4. child_stack.rs (добавить Serialize bound, SavedStack, новые методы)
5. persistent_state.rs (новый трейт в core)
   ↓
6. component_node.rs (blanket-impl PersistentState → ComponentNode)
   ↓
7. navigation_host.rs (save/restore в showcase)
8. app.rs (подключить в ShowcaseApplication)
   ↓
9. nested.rs (рекурсивное сохранение)
10. state_screen.rs (пример кастомных данных)
   ↓
11. Тесты (child_stack, saved_state, integration)
12. Документация (обновить SKILL.md)

---

## Шаг 13. JNI-мост для kill/restore процесса

### Задача

Передавать Vec<u8> (сериализованный SavedStack<C>) между Rust и Android Bundle
через JNI, чтобы пережить убийство процесса.

### Архитектура (два сценария)

**Config Change (поворот) — процесс жив:**
- InitWindow → on_restore_state(None)
- Application берёт из self.saved_state (своё поле)
- PlatformState buffer НЕ используется

**Kill/Restore — процесс пересоздан:**
- onCreate → JNI → PlatformState.set_saved_state(bytes)
- InitWindow → PlatformState.take_saved_state() → app.on_restore_state(Some(bytes))

### Пошаговый план

#### 13.1. Kotlin — EguiActivity.kt

Файлы:
- examples/showcase/android/.../EguiActivity.kt
- examples/counter/android/.../EguiActivity.kt
- crates/platform-android/kotlin/EguiActivity.kt

Добавить native-методы nativeGetSavedState()/nativeSetSavedState(),
onSaveInstanceState/onCreate.

#### 13.2. Rust — PlatformState buffer

Файл: crates/platform-android/src/platform_state.rs

Добавить saved_state_buffer: Option<Vec<u8>>.
Методы set_saved_state/take_saved_state.

#### 13.3. Rust — JNI-функции (#[no_mangle])

Файл: crates/platform-android/src/saved_state_jni.rs (НОВЫЙ)

Глобальный OnceLock<PlatformState> для доступа из UI thread.
Функции nativeGetSavedState/nativeSetSavedState.

#### 13.4. Rust — lifecycle связка

Файл: crates/platform-android/src/lifecycle.rs

- handle_stop/destroy: после on_save_state() → platform_state.set_saved_state(bytes)
- handle_init_window: platform_state.take_saved_state() → app.on_restore_state(Some(bytes))
- Добавить параметр &PlatformState в lifecycle-функции

#### 13.5. Rust — главный цикл

Файл: crates/platform-android/src/loop.rs

Проброс &PlatformState в handle_lifecycle_event.

#### 13.6. Application — проверка

Файл: examples/showcase/src/app.rs

Изменений не требуется: on_restore_state(Some(bytes)) и on_save_state() уже готовы.

#### 13.7. Тестирование на устройстве с логами

1. Собрать showcase APK
2. Запустить, перейти на State Screen, изменить counter
3. Повернуть экран — проверить логи
4. adb shell am kill com.example.egui_android — убить процесс
5. Перезапустить — проверить логи

Логи для верификации:
  on_save_state: сохранено N элементов (M байт)
  PlatformState: set_saved_state, M байт
  JNI: nativeGetSavedState M байт

После kill:
  JNI: nativeSetSavedState M байт
  PlatformState: take_saved_state M байт
  on_restore_state: восстанавливаем N элементов

#### 13.8. Документация

Обновить SKILL.md. Описать JNI-мост и kill/restore поток.

### Результирующая файловая структура (шаг 13)

```
crates/platform-android/
  src/
    saved_state_jni.rs     ← NEW: JNI-функции
    platform_state.rs      ← CHANGE: поле saved_state_buffer
    lifecycle.rs           ← CHANGE: связка буфера
    loop.rs                ← CHANGE: проброс PlatformState
    lib.rs                 ← CHANGE: pub mod saved_state_jni
  kotlin/
    EguiActivity.kt        ← CHANGE: native-методы

examples/
  showcase/android/.../EguiActivity.kt  ← CHANGE: JNI-методы
  counter/android/.../EguiActivity.kt   ← CHANGE: JNI-методы
```

### Порядок подшагов

```
13.1 Kotlin: EguiActivity.kt
        ↓
13.2 Rust: PlatformState buffer
        ↓
13.3 Rust: JNI #[no_mangle] функции
        ↓
13.4 Rust: lifecycle связка
        ↓
13.5 Rust: loop.rs связка
        ↓
13.6 Rust: Application (проверка, без изменений)
        ↓
13.7 Тестирование на устройстве
        ↓
13.8 Документация
```

### Критические уточнения

#### OnceLock для PlatformState

JNI-функции вызываются из UI thread в onCreate (до создания backend'а).
PlatformState нужно инициализировать в глобальный OnceLock:

```rust
// platform_state.rs
use std::sync::OnceLock;
static GLOBAL_PLATFORM_STATE: OnceLock<PlatformState> = OnceLock::new();

impl PlatformState {
    pub fn new() -> Self {
        let state = Self { inner: Arc::new(Mutex::new(PlatformStateInner::default())) };
        GLOBAL_PLATFORM_STATE.set(state.clone()).ok();
        state
    }
    pub fn set_saved_state(&self, bytes: Option<Vec<u8>>) { ... }
    pub fn take_saved_state(&self) -> Option<Vec<u8>> { ... }
}

// saved_state_jni.rs: JNI-функции используют GLOBAL_PLATFORM_STATE.get()
```

#### Связка on_save_state() -> PlatformState

handle_stop и handle_destroy перестают игнорировать результат on_save_state(),
а передают его в PlatformState:

```rust
fn handle_stop<A: Application>(app_instance: &mut A, ps: &PlatformState) {
    let saved = app_instance.on_save_state(); // SavedState = Option<Vec<u8>>
    ps.set_saved_state(saved);                // buffer for JNI/kill/restore
    app_instance.on_stop();
}
```

#### Связка handle_init_window -> PlatformState.take_saved_state()

handle_init_window берёт bytes из PlatformState вместо жёсткого None:

```rust
fn handle_init_window<A: Application>(..., ps: &PlatformState) {
    // ... EGL init ...
    let saved = ps.take_saved_state();
    app_instance.on_restore_state(saved);
}
```

### Проверка ShowcaseApplication

on_restore_state уже написан по паттерну:
  let bytes = state.or_else(|| self.saved_state.take());

Покрывает оба сценария:
- Config change: state = Some(bytes) из PlatformState, использует напрямую
- Kill/restore: state = Some(bytes) из PlatformState (JNI), использует напрямую
- Первый запуск: state = None, fallback на self.saved_state (None), ничего

### Полная схема Rust-части

```
Создание (run.rs):
  backend создаёт PlatformState::new()
  -> PlatformState сохраняется в GLOBAL_PLATFORM_STATE (OnceLock)

Главный цикл (loop.rs):
  tick(...) -> handle_lifecycle_event(ev, ..., &platform_state)

Lifecycle (lifecycle.rs):
  Stop:
    saved = app.on_save_state()           -> Vec<u8>
    ps.set_saved_state(Some(saved))       -> buffer
    app.on_stop()

  Destroy:
    saved = app.on_save_state()           -> Vec<u8>
    ps.set_saved_state(Some(saved))       -> buffer

  InitWindow:
    saved = ps.take_saved_state()         <- buffer
    app.on_restore_state(saved)           -> Application

JNI (saved_state_jni.rs):
  onSaveInstanceState:
    ps = GLOBAL_PLATFORM_STATE.get()
    bytes = ps.take_saved_state()         -> Vec<u8>
    return byteArray -> Kotlin Bundle

  onCreate:
    byteArray -> Vec<u8>
    ps = GLOBAL_PLATFORM_STATE.get()
    ps.set_saved_state(Some(bytes))       -> buffer
```
