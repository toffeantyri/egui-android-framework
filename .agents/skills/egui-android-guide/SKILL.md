---
name: egui-android-guide
description: Архитектура, правила и идиомы проекта egui-android-framework. Используй при доработке крейта, написании примеров, рефакторинге тестов.
---

# egui-android-framework: Архитектура и правила

Проект — MVI-фреймворк для запуска egui-приложений на Android,
разделённый на 7 крейтов в едином workspace.
Вдохновлён [Decompose](https://github.com/arkivanov/Decompose) и Jetpack Compose:
древовидная архитектура компонентов с однонаправленным потоком данных
и реактивным состоянием.

Сообщения отправляются из View в момент события через `Dispatcher`,
а не собираются в `Vec` и не возвращаются после рендера.

## Архитектура (MVI + Reactive State + Dispatcher)

```
                    ┌──────────────────────┐
                    │   framework          │  umbrella, re-exports
                    └──────┬───────────────┘
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
        ▼                  ▼                  ▼
┌──────────────┐   ┌──────────────┐   ┌──────────────┐
│  navigation  │   │     ui       │   │    core      │
│  ChildStack  │   │  remember    │   │  Component   │
│  Lifecycle   │   │  builders    │   │  ViewFn      │
│              │   │  modifier    │   │  Widget      │
│              │   │  widgets     │   │  Lifecycle   │
│              │   │  containers  │   │  Ctx         │
│              │   │  animation   │   └──────────────┘
│              │   │  theme       │
└──────┬───────┘   └──────┬───────┘
       │                  │
       └──────────────────┼──────────────────┐
                          │                  │
                          ▼                  ▼
                   ┌──────────────────────────────┐
                   │         runtime              │
                   │  Application | Dispatcher    │
                   │  StateStore | UiNotifier     │
                   │  AppConfig | AppError        │
                   └──────────────┬───────────────┘
                                  │
                    ┌─────────────┴─────────────┐
                    │                           │
                    ▼                           ▼
          ┌─────────────────┐       ┌──────────────────────┐
          │    platform     │       │  platform-android    │
          │  Platform trait │◀──────│  EGL | NDK input    │
          │  PlatformEvent  │       │  run<A: Application> │
          │  FrameInput/Out │       │  egui_glow renderer  │
          │  PlatformConfig │       └──────────────────────┘
          └─────────────────┘
```

**Граф зависимостей (DAG, без циклов):**
```
platform-android → platform, runtime
runtime          → platform (Waker), egui, tokio
core             → runtime
ui               → core
navigation       → core, ui
framework        → core, ui, navigation, runtime, platform, platform-android
```

## Поток данных (реактивный с Dispatcher)

```
UI (нажатие кнопки)
  → dispatch.dispatch(Msg::Increment)           ← в момент клика
    → Receiver (mpsc) накапливает сообщения
      ← после render: drain receiver
        → Component::handle(msg) — отправляет команду в data layer
          → Data Layer (фоновый поток) получает команду
            → store.update(|s| s.count += 1)    ← ЕДИНСТВЕННАЯ ТОЧКА
              → notify_tx.send(())               ← сигнал в Runtime
                → UiNotifier::check()
                  → ctx.request_repaint()
                  → waker.wake()                    ← Waker из platform (app.run_on_java_main_thread)
                    → poll_events() получает Wake
                      → frame() → render(state, &dispatcher) → новый UI
```

Никакого polling. Никаких `poll()`, `on_event()`, `needs_redraw`.
Состояние само уведомляет Runtime через `notify_tx`.

## Упрощённая MVI-модель

В проекте используется упрощённая модель, где **Intent и Message — это одно и то же**.

В классической MVI (как описано в `android-egui-architecture`):
```
Intent (сырое событие UI) → Message (семантическое действие) → Reducer
```

У нас сразу:
```
Message = и событие, и семантика
```

Пример:
```
Клик по кнопке "+" → dispatch(Msg::Increment) → handle(Msg::Increment) → reducer
```

Это осознанное упрощение для снижения бойлерплейта.
Архитектурный контракт (`android-egui-architecture`) описывает строгую модель с Intent,
но в коде проекта Intent и Message объединены.

Если проект вырастет и потребуется разделение (например, для валидации Intent
до превращения в Message) — введём отдельный тип Intent.

**Правило для агента:** не вводи разделение Intent/Message без явного указания.
Используй `Message` (или `Msg`) как единый тип для событий UI.

## Трейты и их расположение по крейтам

### `Application` — `egui-android-runtime`
- Корень DI. Владеет RootComponent, каналами, AppConfig.
- `create()` — создаёт приложение, каналы, запускает data layer.
- `root() / root_ref()` — доступ к корневому компоненту.
- `config() / config_mut()` — настройки (log_tag, target_fps).
- `create_notifier(ctx, wake: Waker) -> Option<UiNotifier>` — создаёт инфраструктуру уведомлений.
  `Waker` — из `egui-android-platform`, создаётся в backend через `app.run_on_java_main_thread(||{})`.
- `frame(ctx, raw_input) -> FullOutput` — один кадр: создаёт `Dispatcher`, вызывает `sync → render`, drain'ит receiver, обрабатывает сообщения через `handle()`.
- `on_save_state()` — вызывается платформой при Lifecycle::Destroy. Сохранить ChildStack.
- `on_restore_state()` — вызывается платформой при InitWindow. Восстановить ChildStack.
- **Не наследует** `LifecycleObserver` (планируется, сейчас методы объявлены прямо в трейте).

### `Dispatcher<M>` — `egui-android-runtime`
- Абстракция над `std::sync::mpsc::Sender`. Скрывает канал от View.
- `new() -> (Self, Receiver<M>)` — создаёт пару dispatcher/receiver.
- `dispatch(msg)` — отправляет сообщение в момент события (клик, переключение и т.д.).
- `Clone` — можно передавать дочерним компонентам и в замыкания.
- View не знает про канал внутри — только про `dispatch()`.
- Имеет две внутренние реализации: `DirectImpl` (прямая отправка) и `WrappedImpl` (упаковка M в `Box<dyn Any + Send>` для совместимости с `DynDispatcher`).

### `DynDispatcher` — `egui-android-runtime`
- Type-erased диспетчер сообщений. Хранит `mpsc::Sender<Box<dyn Any + Send>>`.
- `new() -> (Self, Receiver<Box<dyn Any + Send>>)` — создаёт пару.
- `wrap::<M>() -> Dispatcher<M>` — создаёт типизированный `Dispatcher`, который упаковывает M без спавна потоков.
- Используется в `ComponentNode::render()` для передачи `Dispatcher<M>` экранам.

### `Component` — `egui-android-core`
- Узел дерева навигации. Хранит snapshot состояния + ссылку на Store.
- `render(ui: &mut UiWrapper, &dispatcher)` — делегирует View-функции. Принимает `UiWrapper` (не `egui::Ui`).
- `handle(msg)` — обрабатывает сообщение (отправляет команду в data layer).
- `sync_from_store()` — синхронизирует snapshot из Store (вызывается перед render).
- `state() -> &State` — текущий snapshot для View.
- `type Message: 'static + Send` — сообщение должно быть Send для передачи через каналы.
- Наследует `LifecycleObserver`.

### `ComponentNode` — `egui-android-core`
- Object-safe трейт для хранения разнотипных компонентов в `Box<dyn ComponentNode>`.
- Аналог `Component<*, *>` из Decompose (type erasure).
- `render(ui: &mut UiWrapper, dispatch: &DynDispatcher)` — type-erased рендер.
- `handle_dyn(msg: Box<dyn Any + Send>)` — type-erased handle (downcast внутри).
- `handle_back() -> bool` — встроенная поддержка BackPressed (как в Decompose).
- `save_state() -> Option<Box<dyn Any + Send>>` — сохранить состояние компонента
  для восстановления после пересоздания (по умолч. None).
- `restore_state(Box<dyn Any + Send>)` — восстановить ранее сохранённое состояние.
- `as_any() / as_any_mut()` — downcast для тестирования и доступа к конкретному типу.
- Blanket-impl: любой `Component<Message: 'static + Send>` автоматически становится `ComponentNode`.

### `StateStore<T>` — `egui-android-runtime`
- Реактивное состояние на `tokio::sync::watch`.
- `update(f)` — атомарно изменить состояние (+ уведомить подписчиков).
- `dispatch(msg, reducer)` — MVI-диспатч через reducer.
- `state() -> T` — snapshot текущего состояния.
- `subscribe() -> watch::Receiver<T>` — подписка на изменения.
- `clone_state() -> Self` — копия с разделяемым каналом.

### `UiNotifier` — `egui-android-runtime`
- Инфраструктурный уведомитель. Вызывается в главном потоке.
- `check()` — проверяет `mpsc::Receiver<()>` и при сигнале вызывает `repaint + wake()`.
- Принимает `Option<Waker>` — waker из `egui-android-platform` (создаётся в backend).
- Не знает про Domain, Components, Reducer.

### `Widget<M>` — `egui-android-core`
- Базовый трейт для всех виджетов и модификаторов.
- `render(&self, ui: &mut UiWrapper, dispatch)` — рендерит виджет, может диспатчить сообщения.
- Принимает `&mut UiWrapper` (не `&mut egui::Ui`), что даёт доступ к `Constraints`.
- `UiWrapper` — обёртка над `egui::Ui` с поддержкой `Constraints` (аналог Compose BoxWithConstraints).

### `Modifier<M>` и `ModifierDsl` — `egui-android-ui`
- Value-тип модификатора: `Modifier::new()` — пустой, `.padding(N)`, `.background(C)`, `.fill_max_width()`, `.size(W, H)`, `.clickable(msg)`.
- `ModifierDsl` — extension trait для применения модификатора к виджету: `.modifier(Modifier::new().padding(8.0))`.
- Заменяет старый `ModifierExt` (удалён). Не требует отдельного импорта — `ModifierDsl` реализован blanket-impl для всех `Widget<M>`.

### `AnimationExt<M>` — `egui-android-ui`
- Extension trait для анимаций: `fade(opacity)`, `slide(direction, offset)`.
- Реализован blanket-impl для всех `Widget<M>`.

### `Button<M>`, `Text`, `Spacer`, `Icon` — `egui-android-ui`
- Готовые виджеты, реализующие `Widget<M>`.
- `Button::new(text).on_click(msg)` — при клике диспатчит сообщение (MVI-поток).
- `Button::new(text).on_click_with(closure)` — при клике вызывает closure (локальное UI-действие).
  Можно комбинировать с `on_click()` — сначала msg, потом closure.
- `Button::theme_colors(color)` — pressed вычисляется автоматически (затемняет/осветляет).
- `Button::colors(normal, pressed)` — полный контроль цветов кнопки.
- `Button::text_color(color)` — цвет текста кнопки.
- `Text::new(text)` — отображает строку.
- `Spacer::new(size)` — вертикальный отступ.
- `Icon::new(image)` — отображает изображение.

### `Column`, `Row`, `Stack`, `LazyColumn` — `egui-android-ui`
- Контейнеры, управляющие расположением дочерних виджетов.
- `Column::new().show(ui, dispatch, |ui, dispatch| { ... })` — вертикальное расположение, spacing по умолч. 8.0.
- `Row::new(ui, dispatch, |ui, dispatch| { ... })` — горизонтальное расположение, spacing по умолч. 8.0.
- `Stack::new().add(child1).add(child2).show(ui, dispatch)` — наложение виджетов (builder pattern, список детей).
  Двухфазный measure→layout: consum = max(children), overlay.
  align отложен (ждёт `pub set_cursor`).
- `LazyColumn::new(items).show(ui, dispatch, |item, ui, dispatch| { ... })` — скроллируемый список.
- Контейнеры передают детям constraints: `fill_max_width()` внутри Column растягивает дочерний виджет на всю ширину.

### `AnimatedVisibility<M>`, `Fade<W,M>`, `Slide<W,M>` — `egui-android-ui`
- Виджеты анимаций.
- `AnimatedVisibility::new(visible, duration).child(widget)` — плавное появление/исчезновение.
- `Fade::new(inner, opacity)` — прозрачность.
- `Slide::new(inner, direction, offset)` — смещение.

### `animate_value()`, `animate_bool()` — `egui-android-ui`
- Функции-хелперы интерполяции через `egui::Context`.

### `Theme`, `MaterialTheme`, `ColorPalette`, `Typography`, `Shapes` — `egui-android-ui`
- Система тем.
- `Theme::apply(ctx)`, `Theme::current(ctx)` — установка/чтение темы через `egui::Context::data()`.
- `MaterialTheme::light()` / `MaterialTheme::dark()` — Material Design 3 палитры.

### `remember()`, `RememberState<T>` — `egui-android-ui`
- Локальное UI-состояние между кадрами.
- `remember(ui, key, || init)` — создаёт/восстанавливает состояние.
- Внутри использует `Arc<RwLock<T>>` — методы `set()`, `modify()` принимают `&self` (не `&mut self`).
- Можно использовать внутри замыканий контейнеров (`Column::new`, `Row::new`, `LazyColumn::new`).
- `Clone` через `Arc::clone()` — все клоны разделяют одно состояние.
- `RememberState::get()` — возвращает `RwLockReadGuard<T>` (авторазыменование до `&T`).
- `RememberState::set(value)` — устанавливает новое значение.
- `RememberState::modify(|v| ...)` — изменяет значение через замыкание.

### `ChildStack<C>` — `egui-android-navigation`
- Стек дочерних компонентов с управлением жизненным циклом.
- Generic только по `C` (конфигурация/route). Внутри хранит `Vec<(C, Box<dyn ComponentNode>)>`.
- `push(config, Box<dyn ComponentNode>)` — добавить компонент на вершину.
- `pop() -> Option<(C, Box<dyn ComponentNode>)>` — убрать верхний элемент.
- `active() -> Option<&dyn ComponentNode>` — ссылка на активный компонент.
- `active_mut() -> Option<&mut dyn ComponentNode>` — мутабельная ссылка.
- `replace / bring_to_front / clear` — управление стеком.
- При push/pop/replace вызываются lifecycle-методы компонентов (on_create, on_destroy).
- Аналог `ChildStack` из Decompose. Позволяет хранить компоненты разных типов.
- `save() -> Vec<(C, Option<Box<dyn Any + Send>>)>` — сохранить состояние всех компонентов
  для восстановления после пересоздания Activity.
- `restore(saved)` — передать сохранённое состояние компонентам через `restore_state()`.

### `BackDispatcher`, `BackCallback` — `egui-android-core`
- Центральный менеджер обработки системной кнопки Back.
- `BackDispatcher::register(callback)` — зарегистрировать обработчик с приоритетом.
- `BackDispatcher::handle() -> bool` — вызвать обработчики от высокого приоритета к низкому.
- `BackCallback { priority, handler: Box<dyn FnMut() -> bool> }` — callback, true = Back перехвачен.
- Приоритеты: 100 — диалоги, 10 — компонент, 0 — системное поведение.
- **Внимание:** callback в BackDispatcher может пережить зарегистрировавший его компонент.
  Для экранов с кастомной обработкой Back используйте `handle_back()` через прямой вызов
  из `RootComponent::on_back()`, без BackDispatcher.
- Аналог `BackDispatcher` из Decompose.

### `UiWrapper` и `Constraints` — `egui-android-core`
- `UiWrapper` — обёртка над `egui::Ui`, которая хранит `Constraints` и передаёт их детям.
- `Constraints { min_width, max_width, min_height, max_height }` — Compose-like ограничения размера.
- Методы: `UiWrapper::new(ui, constraints)`, `new_unconstrained(ui)`, `new_child(builder)`, `allocate_space(size)`.
- `Deref<Target = egui::Ui>` — полная совместимость с egui-кодом.
- Вариант `Owned` — для child_ui, решает проблему borrow checker.
- Constraints хранятся гибридно: в поле (type-safe) + `Context::data()` (переживают `Frame::show`).

### `ComponentContext<NavEvent, DataCmd, State>` — `egui-android-core`
- Контекст компонента, передаётся при создании.
- `back_dispatcher` — обработка системной кнопки Back.
- `back_fallback` — fallback при pop из ChildStack.
- `finish_requested` — флаг завершения приложения.
- `saved_state: Option<Box<dyn Any + Send>>` — сохранённое состояние для восстановления
  после пересоздания Activity. Устанавливается из `ComponentNode::save_state()`,
  передаётся в `restore_state()`.

### `LifecycleObserver` — `egui-android-core`
- Трейт с методами `on_create / on_start / on_resume / on_pause / on_stop / on_destroy`.
- Все методы имеют пустую реализацию по умолчанию.

## View — чистая функция с Dispatcher

```rust,ignore
type ViewFn<S, M> = fn(state: &S, ui: &mut UiWrapper, dispatch: &Dispatcher<M>);
```

View не хранит состояние, не знает о каналах, не имеет побочных эффектов.
Сообщения отправляются через `dispatch.dispatch(msg)` в момент события,
а не возвращаются списком после рендера.

Функция легко тестируется и переиспользуется.

### Пример View в Compose-like стиле

```rust,ignore
fn counter_view(state: &u32, ui: &mut UiWrapper, dispatch: &Dispatcher<Msg>) {
    // remember внутри замыкания — работает благодаря Arc<RwLock<T>>
    Column::new().show(ui, dispatch, |ui, dispatch| {
        let show_details = remember(ui, "show_details", || false);

        Text::new("egui Counter")
            .modifier(Modifier::new().padding(8.0))
            .render(ui, dispatch);
        Spacer::new(16.0).render(ui, dispatch);

        Text::new(format!("{}", state))
            .modifier(
                Modifier::new()
                    .padding(16.0)
                    .background(egui::Color32::from_gray(40))
            )
            .render(ui, dispatch);

        Button::new("+1")
            .on_click(Msg::Increment)
            .modifier(
                Modifier::new()
                    .padding(8.0)
                    .background(egui::Color32::from_rgb(0, 128, 255))
            )
            .render(ui, dispatch);

        // Локальное UI-действие через on_click_with
        Button::new(if *show_details.get() { "Скрыть" } else { "Показать" })
            .on_click_with({
                let show_details = show_details.clone();
                move |_ui, _dispatch| show_details.modify(|v| *v = !*v)
            })
            .modifier(Modifier::new().fill_max_width())
            .render(ui, dispatch);

        AnimatedVisibility::new(*show_details.get(), 0.3)
            .child(
                Text::new("Анимированный текст")
                    .modifier(Modifier::new().padding(12.0))
            )
            .render(ui, dispatch);
    });
}
```

## Каналы

| Канал | Тип | Откуда → Куда |
|---|---|---|
| `dispatcher` / `receiver` | `mpsc::channel::<Msg>()` | View → Component (через Dispatcher) |
| `cmd_tx` / `cmd_rx` | `mpsc::channel::<Msg>()` | Component::handle → Data Layer |
| `notify_tx` / `notify_rx` | `mpsc::channel::<()>()` | Data Layer → UiNotifier |
| `store` | `StateStore<T>` (watch) | Data Layer → Component (sync_from_store) |

Dispatcher создаётся каждый кадр в `frame()` и живёт один кадр.
Receiver drain'ится после render — все сообщения обрабатываются через `handle()`.

## Типовой frame() в Application (с DynDispatcher)

В Decompose-like архитектуре RootComponent не реализует `Component`.
Рендер идёт через `render_dyn()` с `DynDispatcher`.
Сообщения от виджетов упаковываются в `Box<dyn Any + Send>` и читаются из `dyn_receiver`.

```rust,ignore
fn frame(&mut self, egui_ctx: &egui::Context, raw_input: egui::RawInput) -> egui::FullOutput {
    self.root.sync_from_store();

    let (dyn_dispatcher, dyn_receiver) = DynDispatcher::new();

    let full_output = egui_ctx.run_ui(raw_input, |ctx| {
        egui::CentralPanel::default()
            .frame(egui::Frame::new()
                .fill(egui::Color32::TRANSPARENT)
                .inner_margin(egui::Margin::ZERO)
                .outer_margin(egui::Margin::ZERO))
            .show(ctx, |ui| {
                let mut wrapper = UiWrapper::new_unconstrained(ui);
                self.root.render_dyn(&mut wrapper, &dyn_dispatcher);
            });
    });

    for msg in dyn_receiver.try_iter() {
        if let Ok(root_msg) = msg.downcast::<RootMsg>() {
            self.root.handle_msg(*root_msg);
        }
    }

    full_output
}
```

После `frame()` рантайм (platform-android) проверяет `request_destroy()`:
```rust,ignore
let full_output = app_instance.frame(&egui_ctx, raw_input);
if app_instance.request_destroy() {
    destroy_requested = true;
}
```
`destroy_requested` вынесен из тела цикла — он сохраняет значение между итерациями.

## BackPressed — обработка платформенной кнопки Назад

BackPressed обрабатывается по архитектуре Decompose: иерархический перехват с fallback на `pop()`.

### ВАЖНО: Безопасность указателей

`BackDispatcher` хранит callback'и, которые **переживают** зарегистрировавший их компонент.
Если компонент уничтожается (pop из корневого ChildStack) — callback в BackDispatcher становится
висячим указателем → **SIGSEGV**.

**Правило:** Не регистрировать callback в BackDispatcher, если он ссылается на состояние компонента,
которое может быть уничтожено раньше, чем callback будет вызван.

Для экранов с кастомной обработкой Back используется **прямой вызов `handle_back()`**
из `RootComponent::on_back()`, без BackDispatcher.

### Цепочка обработки

```
Android Back (AKEYCODE_BACK)
  ↓
platform-android/input.rs (перехват, InputStatus::Handled)
  ↓
run.rs: app_instance.on_back_pressed()
  ↓
Application::on_back_pressed()
  ↓
RootComponent::on_back()
  ├── BackCustomScreen::handle_back()?
  │   └── true → кастомная логика (переключение цвета)
  ├── NestedScreen::handle_back()?
  │   ├── вложенный ChildStack не пуст → pop() → true
  │   └── вложенный ChildStack пуст → false
  └── ComponentContext::on_back()
      ├── BackDispatcher::handle()
      │   └── зарегистрированные callback'и (диалоги, BottomSheet)
      ├── back_fallback
      │   ├── стек > 1 → pop() из Root ChildStack
      │   └── стек = 1 (Home) → false (завершение приложения)
      └── RootComponent проверяет: если стек не изменился и Back не обработан → завершение
```

### Как добавить экран с кастомной обработкой Back

1. В структуре экрана реализовать `handle_back(&mut self) -> bool`
2. Экран должен реализовать `ComponentNode` (напрямую или через blanket-impl от `Component`)
3. В `RootComponent::on_back()` проверить активный компонент через `as_any_mut().downcast_mut::<MyScreen>()`

Пример — `BackCustomScreen` (переключает цвет фона при Back вместо pop):

```rust,ignore
pub struct BackCustomScreen {
    bg: BgColor,
}

impl BackCustomScreen {
    pub fn handle_back(&mut self) -> bool {
        match self.bg {
            BgColor::Blue => { self.bg = BgColor::Green; true }
            BgColor::Green => false, // второй Back уходит на Root
        }
    }
}

// BackCustomScreen реализует Component — ComponentNode через blanket-impl
impl Component for BackCustomScreen { ... }
```

В `RootComponent::on_back()`:
```rust,ignore
if let Some(active) = self.stack.active_mut() {
    if let Some(custom) = active.as_any_mut().downcast_mut::<BackCustomScreen>() {
        if custom.handle_back() {
            return;
        }
    }
}
```

`ScreenComponent` (enum-агрегатор) больше не существует — удалён в пользу Decompose-like архитектуры.

### Правила

- **Platform не знает про Navigation** — только выставляет флаг `back_pressed`.
- **Application не принимает решений** — только делегирует `RootComponent::on_back()`.
- **RootComponent решает** — проверить активный компонент, pop или завершение.
- **Активный компонент может перехватить Back** через `handle_back()`, вернув `true`.
- **Home — последний экран** — Back на Home = завершение приложения (`destroy_requested = true`).
- **UI-кнопка «← Назад»** — диспатчит `RootMsg::Back`, который вызывает `on_back()` (тот же путь).
- **Не каналы** — ChildStack больше не содержит `back_handlers: Vec<Vec<mpsc::Sender<()>>>`.
  Только прямой вызов `handle_back()`.

### `BackDispatcher`

Для будущих сценариев (диалоги, BottomSheet) используется `BackDispatcher`:
- Хранит обработчики с приоритетами.
- `register(BackCallback { priority, handler })` — компонент регистрирует перехват.
- `handle()` — вызывает от высокого приоритета к низкому.
- Первый, кто вернул `true`, перехватывает Back.
- **Внимание:** Не хранить в callback'ах ссылки на компоненты, которые могут быть уничтожены
  раньше вызова callback'а. Для экранов использовать `handle_back()` напрямую.

## Android Lifecycle (как обрабатывается в platform-android)

| Событие | Действие |
|---|---|
| `InitWindow` (первый) | `backend.init()` + `init_graphics()`, `update_system_insets()`, `on_restore_state()` |
| `InitWindow` (повторный) | `recreate_surface()`, не трогать display/context/painter |
| `Resume` | `app_instance.on_resume()` |
| `Pause` | `app_instance.on_pause()` |
| `Stop` | `app_instance.on_stop()` — не чистить EGL |
| `Destroy` | `on_save_state()`, `app_instance.on_destroy()`, destroy painter/EGL, `break` |

### Завершение приложения

```rust,ignore
// run.rs — главный цикл
let mut destroy_requested = false;
loop {
    // poll + input + on_back_pressed + render

    let full_output = app_instance.frame(&egui_ctx, raw_input);

    // После frame проверяем, не запросило ли приложение завершение (см. BackPressed)
    if app_instance.request_destroy() {
        destroy_requested = true;
    }

    if destroy_requested {
        app_instance.on_save_state();  // ← сохраняем навигацию
        app_instance.on_destroy();
        // destroy EGL
        break;
    }
}
```

## Структура проекта (workspace)

```
/
├── Cargo.toml              — workspace root
├── .agents/                — агентские скилы
│
├── crates/
│   ├── platform/           — egui-android-platform
│   │   └── src/
│   │       ├── platform.rs  — trait Platform
│   │       ├── event.rs     — PlatformEvent<W, I>
│   │       ├── frame.rs     — FrameInput, FrameOutput
│   │       └── config.rs    — PlatformConfig
│   │
│   ├── platform-android/   — egui-android-platform-android (cfg = android)
│   │   └── src/
│   │       ├── run.rs       — run<A: Application>() — главный цикл
│   │       ├── egl_backend.rs — EGL FFI + EglState
│   │       ├── input.rs     — InputState + process_input_events()
│   │       ├── waker.rs     — Waker (реэкспорт из egui-android-platform)
│   │       └── ...
│   │
│   ├── runtime/            — egui-android-runtime
│   │   └── src/
│   │       ├── application.rs   — trait Application + AppConfig + DataLayerHandle
│   │       ├── dispatcher.rs    — Dispatcher<M> (Direct/Wrapped)
│   │       ├── dyn_dispatcher.rs — DynDispatcher (type-erased dispatcher)
│   │       ├── store.rs         — StateStore<T>
│   │       ├── ui_notifier.rs   — UiNotifier + AndroidWakeHandle
│   │       └── error.rs         — AppError
│   │
│   ├── core/               — egui-android-core
│   │   └── src/
│   │       ├── component.rs        — trait Component
│   │       ├── component_node.rs   — trait ComponentNode (object-safe, Box<dyn>)
│   │       ├── component_context.rs — ComponentContext (+ BackDispatcher)
│   │       ├── lifecycle.rs        — LifecycleState + LifecycleObserver
│   │       ├── view.rs             — type ViewFn<S, M>
│   │       ├── widget.rs           — trait Widget<M: Send>
│   │       ├── ui_wrapper.rs       — UiWrapper (обёртка над egui::Ui с Constraints)
│   │       ├── constraints.rs      — Constraints { min/max width/height }
│   │       └── back_dispatcher.rs  — BackDispatcher + BackCallback
│   │
│   ├── navigation/         — egui-android-navigation
│   │   └── src/
│   │       └── child_stack.rs — ChildStack<C> (только C, хранит Box<dyn ComponentNode>)
│   │       ├── lib.rs
│   │       ├── remember.rs  — RememberState + remember()
│   │       ├── modifier.rs  — Modifier value type, ModifierNode, ModifierDsl
│   │       ├── widgets/     — Button, Text, Spacer, Icon (все через UiWrapper)
│   │       ├── containers/  — Column, Row, Stack, LazyColumn (передают constraints)
│   │       ├── animation/   — AnimatedVisibility, Fade, Slide, AnimationExt, animate_*
│   │       └── theme/       — Theme, ColorPalette, MaterialTheme, Typography, Shapes
│   │
│   ├── navigation/         — egui-android-navigation
│   │   └── src/
│   │       └── child_stack.rs — ChildStack<C, Comp>
│   │
│   └── framework/          — egui-android-framework (umbrella)
│       └── src/
│           └── lib.rs — pub use всех крейтов
│
└── examples/
    ├── counter/            — реактивный счётчик (Compose-like view)
    └── showcase/           — витрина с навигацией (8 экранов)
```

## Зависимости (ключевые)

| Крейт | Где используется | Зачем |
|---|---|---|
| `egui` 0.31 | runtime, core, ui, platform | GUI-фреймворк |
| `egui_glow` 0.31 | platform-android | Рендеринг через OpenGL |
| `glow` 0.16 | platform-android | OpenGL context |
| `android-activity` 0.6 | platform-android | NativeActivity (только Android) |
| `ndk` 0.9 | platform-android | NDK bindings (NativeWindow) |
| `libc` 0.2 | platform-android | dlopen/dlsym для GL |
| `tokio` 1 (sync) | runtime | watch::channel для StateStore |
| `thiserror` 2.0 | runtime | AppError derive |
| `log` 0.4 | platform-android, runtime | Логирование |
| `android_logger` 0.14 | platform-android | Логирование в logcat |

Нет `crossbeam`, нет `OnceLock` — всё на стандартных `mpsc` + `tokio::sync::watch`.

## Правила взаимодействия с агентом

### Коммиты и push

Агент **НЕ** выполняет `git commit` или `git push` без явного разрешения пользователя.

Перед коммитом агент обязан:
1. Показать `git diff --stat` или summary изменений.
2. Показать предлагаемое сообщение коммита.
3. Спросить: «Коммит?» и дождаться подтверждения.

После коммита агент **НЕ** выполняет `git push` без отдельного разрешения.

## Патч egui (patches/egui)

Проект использует `[patch.crates-io]` в корневом Cargo.toml для наложения
патча на egui 0.31. Исходники патча — в `patches/egui/`, копия оригинала.

**Что исправлено:**

1. **`input_state/mod.rs`** — в `PointerState::begin_pass()` при
   `PointerButton { pressed: true }` устанавливается `old_pos = Some(pos)`,
   чтобы `pointer.delta()` на первом кадре нового drag была нулевой.
   Без этого delta = `new_pos - old_pos_from_UP` могла быть сотнями пикселей.

2. **`containers/scroll_area.rs`** — при `drag_started()` сбрасываются
   `state.vel` и `smooth_scroll_delta`, чтобы остаточная инерция не
   влияла на новый жест.

**PATCH_NOTES.md** — полная документация патча на русском и английском.

**Удаление патча:** когда egui >= X.Y.Z с аналогичным фиксом будет выпущен,
нужно удалить `[patch.crates-io]` из корневого Cargo.toml, удалить
`patches/egui/` и обновить версию egui.

## Батчинг событий (platform-android)

В `crates/platform-android/src/run.rs` реализована умная упаковка событий
в кадр (`deferred_events`):

- Если в кадре есть `PointerButton { pressed: true }` (DOWN) — **все**
  события, кроме самого Down, откладываются на следующий кадр.
- Это гарантирует, что `pointer.delta()` на первом кадре нового drag
  будет нулевой (см. патч egui выше).
- Move-ивенты, пришедшие в том же кадре что и Down, не попадают в
  обработку до следующего кадра, предотвращая скачок scroll_offset.

**Порядок событий при касании:**
```
Кадр N: PointerButton { pressed: true }  (только Down)
Кадр N+1: PointerMoved(pos_1)             (первый Move)
Кадр N+2: PointerMoved(pos_2)             (последующие Move)
...
Кадр M: PointerButton { pressed: false }  (UP)
```

## Важные правила

1. **Не использовать `OnceLock` / глобальные статические переменные** для хранения Sender'ов.
   Sender хранится в Application или Component, Dispatcher создаётся каждый кадр.
   — *Почему:* `OnceLock` не сбрасывается между запусками процесса.

2. **Не вызывать `std::process::exit(0)`** — фреймворк сам завершает цикл через `break`.

3. **EGL display/context переживает InitWindow** — при смене окна пересоздаётся
   только surface.

4. **Сообщения отправляются через `Dispatcher::dispatch()` в момент события**, а не возвращаются списком.
   View не возвращает `Vec<Message>`, а диспатчит их сразу через `&Dispatcher`.

5. **Все комментарии, логи и строки ошибок — на русском.**

6. **`store.update()` — единственная точка изменения состояния.** Никаких прямых
   присваиваний через `RwLock`.

7. **Data layer после `store.update()` всегда шлёт `notify_tx.send(())`.**

8. **Component синхронизируется из store через `sync_from_store()` в начале frame.**

9. **Главный цикл событийный:** `poll_events(16ms)` + `notifier.check()` + `frame()`.
   Никаких `poll()`, `on_event()`, `needs_redraw`.

10. **Упрощённая MVI-модель:** Intent и Message — это одно и то же (`Msg`).
    Не вводи разделение Intent/Message без явного указания.

11. **Импорты — через umbrella или напрямую:**
    ```rust
    // Через umbrella (если подключен egui-android-framework)
    use egui_android_framework::runtime::{Application, Dispatcher, DynDispatcher};
    use egui_android_framework::platform::Waker;
    use egui_android_framework::core::{Component, ComponentNode, LifecycleObserver, UiWrapper};
    use egui_android_framework::navigation::ChildStack;

    // Напрямую (если подключен только конкретный крейт)
    use egui_android_runtime::Application;
    use egui_android_core::{Component, ComponentNode};
    use egui_android_platform::Waker;
    ```

12. **При изменении кода проверять изоляцию крейтов:**
    - `runtime` не импортирует `core`, `ui`, `navigation`
    - `core` не импортирует `navigation`
    - `ui` не импортирует `navigation`
    - `platform` не импортирует `runtime`, `core`, `ui`, `navigation`

13. **RootComponent не реализует Component** — в Decompose-like архитектуре `RootComponent`
    не реализует `Component` трейт. Рендер через `render_dyn(ui, &DynDispatcher)`.
    Сообщения обрабатываются через `handle_msg(msg)` (`downcast` из `dyn_receiver`).

14. **`NavigableRoute` — sealed class паттерн для навигации:**
    ```rust
    pub enum NavigableRoute {
        Main(Route),          // корневой стек
        Nested(NestedRoute),  // вложенный стек
    }
    
    pub enum RootMsg {
        Navigate(NavigableRoute),  // единый вариант для любой навигации
        Back,
        ToggleTheme,
    }
    ```
    Каждый стек имеет свой Route enum. `ChildStack<Route>` для корневого стека,
    `ChildStack<NestedRoute>` для вложенного. Никаких `matches!(route, Route::NestedA...)`.

13. **Compose-like UI строится через замыкания с UiWrapper:**
    ```rust
    use egui_android_framework::ui::{
        widgets::{Button, Text, Spacer, Widget},
        containers::Column,
        modifier::{Modifier, ModifierDsl},
        remember,
    };

    // Контейнеры принимают &mut UiWrapper, &Dispatcher и closure
    Column::new().show(ui, dispatch, |ui, dispatch| {
        Text::new("Заголовок")
            .modifier(Modifier::new().padding(8.0))
            .render(ui, dispatch);

        // remember внутри замыкания
        let count = remember(ui, "counter", || 0i32);
        Text::new(format!("{}", count.get()))
            .modifier(Modifier::new().padding(8.0))
            .render(ui, dispatch);

        // fill_max_width — растягивается на всю ширину контейнера
        Button::new("Кнопка")
            .on_click(Msg::Clicked)
            .modifier(Modifier::new().fill_max_width().padding(8.0))
            .render(ui, dispatch);

        // on_click_with — локальное UI-действие
        Button::new("+1").on_click_with({
            let count = count.clone();
            move |_ui, _dispatch| count.modify(|c| *c += 1)
        }).modifier(Modifier::new().fill_max_width())
        .render(ui, dispatch);
    });
    ```

## GL-режим (GameActivity)

Проект использует **GameActivity** из Android Game Development Kit для
полноценного GL-режима с IME, SurfaceView и InputConnection.

### Архитектура Android-слоя

```
android/
├── app/
│   ├── src/main/
│   │   ├── java/.../EguiActivity.kt   # GameActivity подкласс
│   │   ├── res/values/styles.xml      # AppCompat.NoActionBar тема
│   │   ├── AndroidManifest.xml        # Активность + lib_name
│   │   └── jniLibs/arm64-v8a/
│   │       └── libegui_gl_app.so      # Rust .so
│   └── build.gradle                   # games-activity 4.4.0
├── build.gradle
├── settings.gradle
└── local.properties
```

### Backend-абстракция

Платформа имеет два backend'а:
- **GlBackend** — основной, использует game-activity API
- **NativeBackend** — fallback (без IME), использует native-activity

```rust
pub trait AndroidBackend {
    fn init(&mut self) -> Result<(), String>;
    fn poll_events(&mut self) -> Vec<BackendEvent>;
    fn show_keyboard(&mut self);
    fn hide_keyboard(&mut self);
    // ...
}
```

IME реализован без JNI через `app.show_soft_input()` / `app.hide_soft_input()`.
Текстовый ввод принимается через `app.text_input_state()` (InputConnection).

### Зависимости для сборки

**Rust:**
```bash
rustup target add aarch64-linux-android
cargo install cargo-ndk
```

**Android SDK (Ubuntu/Debian):**
```bash
sudo apt install android-sdk google-android-ndk-r27d-installer gradle openjdk-17-jdk
```

**SDK компоненты:**
```bash
/path/to/cmdline-tools/bin/sdkmanager --sdk_root=/usr/lib/android-sdk \
  "platforms;android-34" "build-tools;33.0.0"
```

### Сборка и запуск

```bash
# 1. Сгенерировать android/ (Gradle-проект)
APP_LABEL="My App" APP_PACKAGE=com.example.myapp APP_LIB_NAME=my_app \
  cargo run --bin cargo-android-init

# 2. Собрать Rust .so
ANDROID_NDK_HOME=/usr/lib/android-sdk/ndk/27.3.13750724 \
  cargo ndk -t arm64-v8a -o android/app/src/main/jniLibs build -p my-app

# 3. Собрать APK
cd android
APP_LIB_NAME=my_app APP_PACKAGE=com.example.myapp ./gradlew assembleDebug

# 4. Установка и запуск
adb install -r app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n com.example.myapp/com.example.egui_android.EguiActivity
```

Или одной командой через `run_android.sh` из примеров.

### Проверка кода

```bash
# Все крейты workspace (хост)
cargo check --workspace
cargo test --workspace

# Только конкретный крейт
cargo test -p egui-android-runtime
cargo test -p egui-android-navigation
```
