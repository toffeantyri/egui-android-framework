---
name: android-egui-architecture
description: Архитектурные требования к построению приложения
disable-model-invocation: false
---

Architecture Skill

Rust Reactive MVI/UDP Architecture for Android + egui

---

Purpose

Этот документ является архитектурным контрактом проекта.

Перед выполнением любой задачи агент обязан:

1. определить, какие слои затрагиваются;
2. проверить соблюдение архитектурных правил;
3. предложить решение, не нарушающее границы ответственности.

Если предлагаемое решение нарушает данный документ, агент обязан отказаться от него и предложить архитектурно корректную альтернативу.

---

Архитектурная философия

Проект использует однонаправленный поток данных (UDP / MVI).

Любое изменение данных происходит только через поток:

Intent
    ↓
Message
    ↓
Reducer
    ↓
State
    ↓
UI

Обратных связей не существует.

UI никогда самостоятельно не изменяет State.

Data Layer никогда не взаимодействует с UI.

---

Главный принцип

Каждый слой отвечает только за одну ответственность.

Никакой слой не должен выполнять работу другого слоя.

---

Соответствие слоёв и крейтов

Проект разделён на 7 крейтов в едином workspace.
Каждый крейт реализует один или несколько архитектурных слоёв:

| Архитектурный слой | Крейт | Ответственность |
|---|---|---|
| Platform | `egui-android-platform` | Абстрактный контракт платформы |
| Platform (Android impl) | `egui-android-platform-android` | Android lifecycle, EGL, input, event loop |
| Runtime | `egui-android-runtime` | Application, Dispatcher, StateStore, UiNotifier |
| Application | `egui-android-runtime` | Application trait, frame(), DI корень, on_save_state/on_restore_state |
| Component | `egui-android-core` | Component trait, ComponentContext, lifecycle |
| UI | `egui-android-core` + `egui-android-ui` | ViewFn, Widget, remember, builders, modifier, widgets, containers, animation, theme |
| State | `egui-android-runtime` | StateStore (tokio::sync::watch) |
| Reducer | `egui-android-runtime` | store.dispatch(msg, reducer) |
| Navigation | `egui-android-navigation` | ChildStack, save/restore состояния |
| Infrastructure | `egui-android-runtime` + `egui-android-core` | Dispatcher, UiNotifier, RuntimeContext, каналы |
| Macros | `egui-android-macros` | #[derive(Component)] — генерация PersistentState |
| Umbrella | `egui-android-framework` | Re-export всех крейтов |

---

## Saved State (Decompose-style)

### PersistentState

**Крейт**: `egui-android-core`

Трейт `PersistentState` — аналог `StateKeeper` в Decompose. Реализуется компонентом
для сохранения кастомных данных при пересоздании Activity.

```rust
pub trait PersistentState {
    type State: Serialize + DeserializeOwned + Send + 'static;
    fn save(&self) -> Self::State;
    fn restore(&mut self, state: Self::State);

    // Хелперы: сериализация/десериализация через bincode
    fn save_to_boxed(&self) -> Option<Box<dyn Any + Send>>;
    fn restore_from_boxed(&mut self, state: Box<dyn Any + Send>);
}
```

### PersistentComponent<T>

**Крейт**: `egui-android-core`

Структурная обёртка, реализующая `ComponentNode` для любого `T: Component + PersistentState`.
Используется в фабрике для автоматического save/restore:

```rust
// В фабрике (ShowcaseFactory):
Route::State => Box::new(PersistentComponent::new(StateScreen::new())),
```

`PersistentComponent<T>` делегирует `render()`, `handle_dyn()`, `as_any()` внутреннему `T`,
и переопределяет `save_state()`/`restore_state()` через `PersistentState::save_to_boxed()`.

**Почему не blanket-impl?** Rust запрещает два blanket-impl для одного трейта.
`PersistentComponent<T>` — compositional solution: обёртка не конфликтует
с blanket-impl `ComponentNode` для `Component`.

### Макрос #[derive(Component)]

**Крейт**: `egui-android-macros`

Генерирует `PersistentState` автоматически по `#[persistent_fields(...)]`:

```rust
#[derive(Component)]
#[persistent_fields(counter, label)]
struct MyScreen {
    counter: i32,   // ← сохраняется
    label: String,  // ← сохраняется
    expanded: bool, // ← НЕ сохраняется (UI-состояние)
}
```

Генерирует скрытую структуру `__MyScreenPersistentState` с `Serialize/Deserialize`
и `impl PersistentState for MyScreen`.

Обязательно обернуть в `PersistentComponent<T>` в фабрике для активации save/restore.

### SavedStack<C>

**Крейт**: `egui-android-runtime`

Сериализуемое представление стека навигации. `ChildStack::save()` возвращает
`SavedStack<C>`, который сериализуется через `bincode` в `Vec<u8>`.

### Поток save/restore

```
Stop/Destroy:
  Platform → app.on_save_state()
  Application → root.save() → SavedStack<C> → bincode → Vec<u8>
  Application → self.saved_state = Some(bytes)  ← Application хранит состояние
  Platform не хранит состояние

InitWindow:
  Platform → app.on_restore_state(None)  ← Platform не имеет данных
  Application → bytes = self.saved_state.take()  ← из своего поля
  Application → bincode → SavedStack<C> → ChildStack::restore_from_saved()
```

**Архитектурное правило:** Platform не хранит состояние приложения.
`RunState` не содержит `saved_state`. Application владеет состоянием.

`restore_from_saved()` пересоздаёт компоненты через фабрику (как в Decompose)
и восстанавливает их состояние через `ComponentNode::restore_state()`.
Для `PersistentComponent<T>` — через `PersistentState::restore_from_boxed()`.

### Правила использования

1. Компонент реализует `Component + PersistentState` (через `#[derive(Component)]` или вручную)
2. В фабрике компонент оборачивается в `PersistentComponent::new(...)`
3. `ChildStack::save()` → вызывает `save_state()` на `PersistentComponent` → `PersistentState::save_to_boxed()` → `Vec<u8>`
4. `ChildStack::restore_from_saved()` → создаёт компонент через фабрику → вызывает `restore_state()`
5. **При kill/restore процесса** нужен JNI-мост (отдельная задача)

---

Слои системы

### Platform Layer

**Крейт**: `egui-android-platform` (абстракция), `egui-android-platform-android` (Android)

Ответственность:

- Android lifecycle
- Android input
- EGL
- OpenGL
- Event Loop
- Window
- Surface

Не знает:

- бизнес-логику
- State
- Reducer
- Components
- runtime, core, ui, navigation

---

### Runtime Layer

**Крейт**: `egui-android-runtime`

Ответственность:

- запуск egui
- создание RawInput
- вызов Context::run()
- передача FullOutput платформе
- Application trait
- Dispatcher, StateStore, UiNotifier
- **RuntimeContext** — контекст выполнения, владеет UiNotifier,
  инкапсулирует Waker. Platform-android вызывает только `check()`.
- **Waker** — платформенная абстракция для пробуждения event loop
  (определён в `egui-android-platform`, передаётся в RuntimeContext через Application)

Не знает:

- бизнес-состояние
- Data Layer
- Domain
- core, ui, navigation

Runtime никогда не принимает решений.

Он только соединяет Platform и UI.

---

### UI Layer

**Крейты**: `egui-android-core` (ViewFn, Widget), `egui-android-ui` (remember, builders, modifier)

Ответственность:

- чтение State
- построение интерфейса
- генерация Intent

UI ничего не знает:

- про сеть
- про БД
- про Android
- про Effects
- про navigation

UI является полностью декларативным.

Запрещено:

repository.load();

или

state.value = ...

из UI.

**Готовые компоненты входят в крейты фреймворка:**
- `egui-android-ui/widgets`: `Button<M>`, `Text`, `Spacer`, `Icon` — все реализуют `Widget<M>`
- `egui-android-ui/containers`: `Column`, `Row`, `Stack`, `LazyColumn` — контейнеры на замыканиях (без generic M)
- `egui-android-ui/animation`: `AnimatedVisibility<M>`, `Fade<W,M>`, `Slide<W,M>`, `AnimationExt<M>`
- `egui-android-ui/theme`: `Theme`, `ColorPalette`, `MaterialTheme`, `Typography`, `Shapes`
- `egui-android-ui`: `remember()`, `ModifierExt<M>`, `AnimationExt<M>`, builders

---

### Component

**Крейт**: `egui-android-core`

Component преобразует Intent в Message.

Он не изменяет состояние (только через `store.update()` из data layer).

Он не обращается к Data Layer напрямую.

Component ничего не рисует — делегирует View-функции.

Component может сохранять/восстанавливать своё состояние через
`save_state() -> Option<Box<dyn Any + Send>>` и `restore_state(Box<dyn Any + Send>)`
для поддержки пересоздания Activity (поворот экрана, kill/restore).

**Направление зависимости:** Component → StateStore (потребитель).
Component использует Store как источник состояния (читает snapshot через `sync_from_store()`).
Обратная зависимость (Store → Component) отсутствует.

---

### Reducer

**Крейт**: `egui-android-runtime` (как функция, передаваемая в `store.dispatch()`)

Единственная точка изменения State.

Любое изменение состояния проходит только через Reducer.

Reducer:

✔ детерминирован

✔ синхронен

✔ не содержит async

✔ не знает про UI

✔ не знает про Android

---

### State

**Крейт**: `egui-android-runtime` (StateStore)

State является immutable snapshot.

View только читает State.

State ничего не знает:

- про UI
- про Android
- про Runtime
- про Data Layer

---

### Effect Layer

Единственное место для:

- async
- network
- database
- filesystem

Effect не изменяет State.

Effect может только отправить Message обратно в Reducer.

Реализуется в приложении (data layer) через фоновые потоки с mpsc-каналами.

---

### Store

**Крейт**: `egui-android-runtime` (StateStore)

Store является единственным владельцем State.

Store отвечает за:

- dispatch
- хранение State
- публикацию нового State

Store не знает:

- Android
- egui
- Components

---

### RuntimeContext

**Крейт**: `egui-android-runtime`

RuntimeContext — контекст выполнения Runtime.

Владеет `UiNotifier` и инкапсулирует `Waker`.
Платформа (platform-android) видит только RuntimeContext и вызывает `check()`.

Отвечает за:

- единую точку входа для платформы (`check()`)
- инкапсуляцию Waker и UiNotifier от платформы

RuntimeContext не знает:

- Domain
- Components
- Reducer
- Data Layer

Архитектурный поток:

```
Platform (Android)
  └── app.create_runtime_context(ctx, waker) → RuntimeContext
  └── rt_ctx.check()                              ← единственный вызов
        └── UiNotifier::check()
              ├── ctx.request_repaint()
              └── waker.wake()
```

---

### UiNotifier

**Крейт**: `egui-android-runtime`

UiNotifier является инфраструктурой.

Он отвечает только за:

- request_repaint()
- wake()

UiNotifier не знает:

- Domain
- Components
- Reducer
- Data Layer

---

Однонаправленный поток данных

Разрешён только следующий поток:

Intent
 ↓
Message
 ↓
Reducer
 ↓
State
 ↓
Store
 ↓
UiNotifier
 ↓
RuntimeContext
 ↓
Runtime
 ↓
UI

Запрещены любые обратные переходы.

---

Граф зависимостей крейтов (DAG)

```
platform-android → platform, runtime
runtime          → platform (Waker), egui, tokio, thiserror, log
core             → runtime
ui               → core
navigation       → core, ui
framework        → core, ui, navigation, runtime, platform, platform-android
```

Циклические зависимости запрещены. Проверка: `cargo tree -e normal`.

---

Запрещённые зависимости

| Слой / Крейт | Не должен знать о |
|---|---|
| UI (core, ui) | Repository, Android, Network |
| Reducer | UI, Runtime, Android, Context |
| Store | Android, egui, Components |
| Effect | UI |
| Platform (абстракция) | Domain, State, Reducer, runtime, core, ui |
| Platform-android (реализация) | runtime — **знает** (это ок: runtime ниже по потоку), Application, StateStore |
| Runtime | core, ui, navigation |
| Core | navigation, ui (builders), platform |

---

Push вместо Pull

Архитектура является полностью событийной.

Запрещено наличие скрытых polling механизмов.

Недопустимо:

- poll()
- update()
- sync()
- refresh()
- check_changes()

если они используются исключительно для обнаружения изменений состояния.

Изменение состояния должно само инициировать уведомление Runtime.

---

Контракт Runtime

Runtime работает только по событиям.

Последовательность:

Platform Event
      ↓
RawInput
      ↓
Context::run()
      ↓
FullOutput
      ↓
Platform Output

Runtime никогда не инициирует изменение State.

---

Контракт egui

egui отвечает только за построение интерфейса.

State не должен управлять жизненным циклом egui.

watch не является системой управления Runtime.

watch используется исключительно как механизм доставки нового состояния.

---

Контракт Android

Android отвечает только за:

- lifecycle
- wake
- input
- window

Android не знает про Store.

Android не знает про Reducer.

Android не знает про Components.

---

Проверка архитектуры

Перед любой реализацией агент обязан проверить:

1. Не нарушается ли однонаправленный поток данных?

2. Не появляется ли новая ответственность у существующего слоя?

3. Не знает ли слой больше, чем должен?

4. Не возникает ли циклическая зависимость?

5. Не превращается ли Runtime в бизнес-слой?

6. Не превращается ли Store в UI слой?

7. Не превращается ли Reducer в сервис?

8. Не содержит ли View бизнес-логики?

9. Не используется ли polling вместо событий?

10. Не нарушается ли контракт egui?

---

Проверка изоляции крейтов (обязательно)

```
✔ Проверка изоляции:
  * platform (абстракция) не видит runtime/core/ui/navigation — [да/нет]
  * platform-android (реализация) видит runtime — [да] (ок, runtime ниже по потоку)
  * runtime не видит core/ui/navigation — [да/нет]
  * runtime видит platform (Waker) — [да] (ок, платформенная абстракция)
  * core не видит navigation — [да/нет]
  * ui не видит navigation — [да/нет]
```

---

Правило "Одна причина изменения"

Каждый тип должен иметь одну причину изменения.

Если изменение Android требует изменения Reducer —

архитектура нарушена.

Если изменение UI требует изменения Platform —

архитектура нарушена.

Если изменение Data Layer требует изменения Runtime —

архитектура нарушена.

---

Правило минимальной связанности

Каждый слой знает только о соседнем слое.

Например:

UI (egui-android-core + egui-android-ui)

↓

Component (egui-android-core)

↓

Store (egui-android-runtime)

↓

Reducer (egui-android-runtime)

↓

Effect (приложение, data layer)

↓

Repository (приложение, data layer)

но никогда не знает о слоях через один или два уровня.

---

Перед каждым ответом

Если задача затрагивает архитектуру, агент обязан сначала выполнить архитектурный аудит.

Ответ начинается с раздела:

Architecture Validation

где перечисляются:

✔ какие слои затрагиваются

✔ какие правила проверены

✔ какие потенциальные нарушения обнаружены

Только после этого допускается предлагать изменения.

---
📐 Правила именования каналов

Общий принцип:
Формат: `<source>_<semantic>_<direction>`
где direction = `tx` (transmit) или `rx` (receive).

Источник (source):
- `ui` — UI слой
- `data` — Data Layer
- `nav` — навигация

Семантика (semantic):
- `msg` — сообщение (Message)
- `cmd` — команда (Command)
- `event` — событие (Event)
- `dynmsg` — type-erased сообщение
- `statechanged` — сигнал об изменении состояния

Правила:
1. Запрещены абстрактные имена: `tx`, `rx`, `cmd`, `notify`, `dispatcher`, `receiver`
2. Имя должно быть самодокументируемым: источник + семантика + направление
3. Суффиксы `_tx`/`_rx` обязательны
4. Имя отражает семантику, а не механику
5. Единообразие во всех крейтах

Семантические категории:

| Категория | tx | rx | Где используется |
|---|---|---|---|
| UI → Component | `ui_msg_tx` | `ui_msg_rx` | Dispatcher |
| Component → Data Layer | `data_cmd_tx` | `data_cmd_rx` | ComponentContext, data layer |
| Data Layer → Runtime | `data_statechanged_tx` | `data_statechanged_rx` | UiNotifier |
| Навигация | `nav_event_tx` | `nav_event_rx` | ComponentContext, ChildStack |
| UI → ComponentNode (type-erased) | `ui_dynmsg_tx` | `ui_dynmsg_rx` | DynDispatcher |

Таблица переименования (было → стало):

| Было | Стало |
|---|---|
| `cmd_tx`/`cmd_rx` | `data_cmd_tx`/`data_cmd_rx` |
| `notify_tx`/`notify_rx` | `data_statechanged_tx`/`data_statechanged_rx` |
| `nav_tx`/`nav_rx` | `nav_event_tx`/`nav_event_rx` |
| `dispatcher`/`receiver` | `ui_msg_tx`/`ui_msg_rx` |
| `dyn_dispatcher`/`dyn_receiver` | `ui_dynmsg_tx`/`ui_dynmsg_rx` |

Примечание: `Dispatcher` и `DynDispatcher` — это типы-обёртки, не каналы. Правила именования применяются к переменным-каналам (Sender/Receiver), а не к типам.

Все новые каналы должны соответствовать этому формату.

И никогда не делаешь коммит, пока тебя о этом прямо не попросили.
