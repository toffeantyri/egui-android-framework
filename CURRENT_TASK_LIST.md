Это — актуальная дорожная карта для выхода фреймворка на продакшн‑уровень.

---

## Статус выполнения (закрытые задачи)

| Задача | Статус | Дата |
|--------|--------|------|
| 🟥 1. Глобальные статики — удалены | ✅ **Выполнено** | До 2026-07-19 |
| 🟥 2. save/restore навигации | ✅ **Выполнено** (базовый) | 2026-07-19 |
| 🟥 3. type-erasure через dyn Any | ✅ **Выполнено** | 2026-07-19 |
| 🟧 4. Монолитный backend | ✅ **Выполнено** | 2026-07-19 |
| 🟨 7. PlatformConfig/AppConfig дублирование | ✅ **Выполнено** | 2026-07-19 |
| 🟨 9. Гибридное хранение PlatformState | ✅ **Выполнено** | 2026-07-19 |
| 🟩 10. DataLayerHandle заглушка | ✅ **Выполнено** | 2026-07-19 |
| 🆕 2b. Router/ComponentFactory слой | ✅ **Выполнено** | 2026-07-20 |

---

## Целевая архитектура: Decompose-style ComponentContext

Текущая архитектура отличается от Decompose в ключевом моменте: **компонент сам владеет ChildStack, сам вызывает save/restore на своих стеках**. В Decompose этой ответственности у компонента нет — всем управляет ComponentContext.

Ниже — целевая архитектура, к которой мы движемся.

---

### Проблема текущей архитектуры

Сейчас:

```
NestedScreen (компонент)
  ├── stack_layer1: ChildStack<NestedRoute>    ← компонент владеет вручную
  ├── stack_layer2: ChildStack<NestedLayer2Route> ← компонент владеет вручную
  ├── save_state_recursive()                    ← компонент пишет вручную
  └── restore_state_recursive()                 ← компонент пишет вручную
```

При добавлении нового вложенного стека нужно:
1. Добавить поле `ChildStack<NewRoute>`
2. Обновить `save_state_recursive()` и `restore_state_recursive()`
3. Обновить структуру состояния

**Это нарушает OCP и требует ручной работы на каждом уровне вложенности.**

---

### Целевая архитектура (Decompose-style)

```
ComponentContext (владелец всего):
  ├── StateKeeper (дерево состояний)     ← save/restore рекурсивно
  ├── ChildStackManager<C>               ← управляет стеком, restore автоматически
  ├── Lifecycle (MergedLifecycle)
  ├── BackHandler
  └── InstanceKeeper

NestedScreen (компонент):
  └── ctx: ComponentContext     ← делегирует всё контексту
  └── navigator: ChildStackManager<NestedRoute>  ← создаётся через ctx

  // save/restore — НЕТ! ComponentContext делает всё автоматически.
```

Ключевые изменения:

1. **`ComponentContext` становится владельцем `StateKeeper`** — дерева состояний, которое автоматически рекурсивно сохраняется и восстанавливается.

2. **`ChildStack` уходит из компонента в `ComponentContext`** — компонент больше не владеет стеками напрямую. Вместо этого `ComponentContext` предоставляет методы для навигации, а управление стеком происходит внутри контекста.

3. **`StateKeeper` — рекурсивное дерево**:
   ```rust
   // Псевдокод
   struct StateKeeper {
       own_state: Option<Box<dyn Any + Send>>,
       children: HashMap<String, StateKeeper>,  // по ключу — дочерние компоненты
   }
   ```
   - При `save()`: StateKeeper сохраняет своё состояние + рекурсивно все `children`
   - При `restore()`: StateKeeper восстанавливает себя + рекурсивно всех `children`
   - Дочерний StateKeeper создаётся через `parent.child(key)` — привязывается к родителю

4. **Компонент регистрирует дочерние контексты через ключи**:
   ```rust
   // В Decompose — при создании дочернего компонента:
   class NestedComponent(
       componentContext: ComponentContext  // родительский контекст
   ) : ComponentContext by componentContext {
   
       // navigator сам создаёт дочерние ComponentContext с их StateKeeper
       private val navigator = ChildrenNavigator(
           source = stackNavigation,
           initialStack = { listOf(ChildConfig.A) },
           childFactory = { config, childCtx ->
               when (config) {
                   is ChildConfig.A -> SubComponent(childCtx)  // ← дочерний контекст
               }
           },
       )
   }
   ```

---

### Поток save/restore в целевой архитектуре

```
Application::on_save_state()
  └── ctx.state_keeper.save()
      ├── сохраняет своё состояние
      └── для каждого child:
          └── child.state_keeper.save()
              ├── сохраняет своё состояние
              └── для каждого child:
                  └── ... рекурсия ...

Application::on_restore_state()
  └── ctx.state_keeper.restore(saved)
      ├── восстанавливает своё состояние
      └── для каждого child по ключу:
          └── child.state_keeper.restore(child_saved)
              ├── восстанавливает своё состояние  
              └── для каждого child:
                  └── ... рекурсия ...
```

Весь save/restore — **автоматический, без единой строки кода в компоненте**.

---

### Что меняется в крейтах

**1. `egui-android-core`** — новый крейт `ComponentContext`:

```
crates/core/src/
  ├── component_context.rs     ← ComponentContext (владелец StateKeeper)
  ├── state_keeper.rs          ← StateKeeper (рекурсивное дерево)
  ├── child_stack_manager.rs   ← ChildStackManager (стек внутри контекста)
  ├── component_node.rs        ← ComponentNode (упрощается — save/restore уходят)
  └── ...
```

**2. `egui-android-navigation`** — упрощается:

```
crates/navigation/src/
  ├── child_stack.rs           ← ChildStack (чистый контейнер, без save/restore)
  ├── component_factory.rs     ← ComponentFactory (остаётся)
  └── ...
```

**3. `egui-android-runtime`** — Application получает `RootComponentContext`:

```
crates/runtime/src/
  ├── application.rs           ← Application владеет RootComponentContext
  └── ...
```

---

### План перехода (Фаза 5)

Переход осуществляется в 4 шага. Каждый шаг — отдельная задача.

#### 🔲 **5a. StateKeeper — рекурсивное дерево состояний** (core)

**Что сделать:**
1. Создать `StateKeeper`:
   ```rust
   pub struct StateKeeper {
       own_state: Option<Box<dyn Any + Send>>,
       children: Vec<(String, StateKeeper)>,
   }
   ```
2. `save() -> StateKeeperSnapshot` — рекурсивно
3. `restore(snapshot)` — рекурсивно
4. `child(key) -> &mut StateKeeper` — создаёт/возвращает дочерний StateKeeper по ключу
5. Тесты: дерево 3 уровня, save/restore, частичное восстановление

**Крейт**: `egui-android-core`
**Зависимости**: нет (чистый core)

---

#### 🔲 **5b. NewComponentContext — владелец StateKeeper** (core)

**Что сделать:**
1. Создать `ComponentContext`:
   ```rust
   pub struct ComponentContext {
       state_keeper: StateKeeper,
       // в будущем: lifecycle, back_handler, instance_keeper
   }
   ```
2. Методы:
   - `save_state()` — делегирует `StateKeeper::save()`
   - `restore_state(snapshot)` — делегирует `StateKeeper::restore()`
   - `child(key) -> ComponentContext` — создаёт дочерний контекст с дочерним StateKeeper
3. `ComponentNode` — больше не содержит `save_state()`/`restore_state()` / `save_state_recursive()` / `restore_state_recursive()`
4. `ComponentNode.render()` — получает `&ComponentContext` вместо `&DynDispatcher` (или наряду с ним)

**Крейт**: `egui-android-core`
**Breaking change**: `ComponentNode` теряет save/restore методы. Компоненты больше не пишут save/restore вручную.

---

#### 🔲 **5c. ChildStackManager — навигация через ComponentContext** (core + navigation)

**Что сделать:**
1. Создать `ChildStackManager<C, M>`:
   ```rust
   pub struct ChildStackManager<C, M> {
       stack: ChildStack<C>,
       ctx: ComponentContext,
       factory: Box<dyn ComponentFactory<C, M>>,
   }
   ```
2. `push(config)`, `pop()`, `replace(config)`, `bring_to_front(config)` — управляют стеком + lifecycle
3. При push — создаёт дочерний `ComponentContext` через `ctx.child(key)`, регистрирует его StateKeeper
4. `save()` — делегирует `ComponentContext::save_state()` (рекурсивно)
5. `restore(saved)` — восстанавливает StateKeeper, создаёт компоненты через фабрику
6. `render(ui, dispatch)` — рендерит активный компонент

**Крейт**: `egui-android-core` + `egui-android-navigation`

---

#### 🔲 **5d. Migration примера на новый ComponentContext** (showcase)

**Что сделать:**
1. `NavigationHost` переходит с `ChildStack` на `ChildStackManager`
2. `NestedScreen` больше не содержит `ChildStack` — только `ChildStackManager<NestedRoute>` и `ChildStackManager<NestedLayer2Route>`
3. `NestedScreen` не переопределяет save/restore — всё делает контекст
4. Удалить `NestedScreenState`, `save_state_recursive()`, `restore_state_recursive()`
5. Проверить: save/restore вложенной навигации без единой строки кода save/restore в компонентах

**Крейт**: `showcase`

---

### Порядок выполнения (обновлённый)

```
✅ Фаза 1 — Выполнено:
  🟥 Задача 1: Глобальные статики — удалены
  🟧 Задача 4: Монолитный backend → модульная архитектура

✅ Фаза 2 — Выполнено:
  🟨 Задача 7: PlatformConfig/AppConfig дублирование (platform + runtime)
  🟩 Задача 10: DataLayerHandle (runtime)
  🟨 Задача 9: Гибридное хранение PlatformState (platform-android)

✅ Фаза 3 — Decompose-совместимость навигации:
  ✅ 🆕 Задача 2b: Router/ComponentFactory (navigation)
  ❌ Задача 2a и 2c — отменены (перекрываются Фазой 5)

🔲 Фаза 4 — Накопившиеся мелкие задачи:
  🔲 🟧 Задача 6: ComponentContext разделение на подконтексты (core)
        — Замена: после Фазы 5 новый ComponentContext заменит старый.
          Пока только минимальные фиксы, если блокируют работу.
  🔲 🟧 Задача 5: Modifier API (ui) — вынести редкие модификаторы
  🔲 🟨 Задача 8: Хрупкие тесты (ui) — тестировать только публичный контракт
  🔲 🟩 Задача 11: Патч egui
  🔲 🟡 Замечание A: Application не наследует LifecycleObserver
  🔲 🟡 Замечание B: MessageEnvelope мёртвый код
  🔲 🟡 Замечание C: JNI указатели в публичном API PlatformState

🔲 Фаза 5 — Decompose-style ComponentContext:
  🔲 Задача 5a: StateKeeper — рекурсивное дерево состояний (core)
  🔲 Задача 5b: NewComponentContext — владелец StateKeeper (core)
  🔲 Задача 5c: ChildStackManager — навигация через ComponentContext (core + navigation)
  🔲 Задача 5d: Migration примера на новый ComponentContext (showcase)
```

---

### Что отменено и почему

| Задача | Решение | Причина |
|--------|---------|---------|
| 2a. Parcelable-сериализация | ❌ Отменена | В Фазе 5 сериализация будет на уровне StateKeeper, не ChildStack. Реализовывать сейчас — делать двойную работу. |
| 2c. Рекурсивное сохранение стеков | ❌ Отменена (текущая реализация некорректна) | Текущая реализация — ручная, не рекурсивная. Правильное рекурсивное сохранение = StateKeeper из Фазы 5. |

---

### Архитектурные замечания (временные)

> Не блокируют Фазу 5, но будут исправлены после неё.

1. **Application не наследует LifecycleObserver** — будет исправлено после редизайна Application.
2. **MessageEnvelope мёртвый код** — удалить после стабилизации DynDispatcher.
3. **JNI указатели в публичном API PlatformState** — будет исправлено при рефакторинге platform-android.
