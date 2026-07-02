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
| Application | `egui-android-runtime` | Application trait, frame(), DI корень |
| Component | `egui-android-core` | Component trait, ComponentContext, lifecycle |
| UI | `egui-android-core` + `egui-android-ui` | ViewFn, Widget, remember, builders, modifier |
| State | `egui-android-runtime` | StateStore (tokio::sync::watch) |
| Reducer | `egui-android-runtime` | store.dispatch(msg, reducer) |
| Navigation | `egui-android-navigation` | ChildStack |
| Infrastructure | `egui-android-runtime` + `egui-android-core` | Dispatcher, UiNotifier, каналы |
| Umbrella | `egui-android-framework` | Re-export всех крейтов |

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

**Готовые компоненты (Button, Text, Spacer и т.д.) не входят в крейты фреймворка.**
Они определяются в приложении с использованием нативных egui-вызовов или `Widget<M>` / `ModifierExt`.

---

### Component

**Крейт**: `egui-android-core`

Component преобразует Intent в Message.

Он не изменяет состояние (только через `store.update()` из data layer).

Он не обращается к Data Layer напрямую.

Component ничего не рисует — делегирует View-функции.

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
Runtime
 ↓
UI

Запрещены любые обратные переходы.

---

Граф зависимостей крейтов (DAG)

```
platform-android → platform, runtime
runtime          → egui, tokio, thiserror, log
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
| Platform | Domain, State, Reducer, runtime, core, ui |
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
  * platform не видит runtime/core/ui/navigation — [да/нет]
  * runtime не видит core/ui/navigation — [да/нет]
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
