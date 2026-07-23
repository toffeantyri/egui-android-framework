Глобально (зачем)

**Пользователь продукта:** навигация, save/restore и Back работают предсказуемо, без unsafe и дублирующихся систем. Альфа-версия готова к стабилизации API.

**Разработчик фреймворка:** получает единственную, завершённую систему навигации. Нет выбора между «старой» и «новой» системой. `ChildStack` — единственный механизм, `ComponentFactory<C>` — единственная фабрика, `PersistentState` — единственный способ сохранения. Back-логика внутри `ChildStack`, без unsafe.

### Задача (что)

Удалить мёртвый код (`ComponentContext2`, `ChildStackManager`, `StateKeeper`, `ComponentState`, `ComponentNode2`), упростить `ComponentContext` (убрать неиспользуемые generic-параметры), перенести Back-логику в `ChildStack`, обновить `NavigationHost`.

### Критерии успеха

- [ ] `ComponentContext2`, `ChildStackManager`, `StateKeeper`, `ComponentState`, `ComponentNode2` — удалены
- [ ] `ComponentContext` — без generic `NavEvent`/`DataCmd`, без `back_fallback` (unsafe), без `navevent_tx`/`datacmd_tx`
- [ ] `ChildStack` — метод `on_back()` с цепочкой: `active.handle_back()` → `pop()`
- [ ] `NavigationHost` — тонкая обёртка над `ChildStack`, без `Box<ChildStack>`, без unsafe-указателей
- [ ] `ShowcaseApplication` — компилируется и работает без изменений в `frame()`/`on_save_state()`/`on_restore_state()`
- [ ] Все 12 тестов навигации (`child_stack_save_tests.rs`) проходят
- [ ] `BackDispatcher` остаётся (для будущих диалогов)

### Границы (не делаем)

- ❌ Не трогаем JNI-мост
- ❌ Не трогаем `PersistentState` / `PersistentComponent<T>`
- ❌ Не трогаем `Application` trait
- ❌ Не трогаем `ComponentFactory<C>` — он нужен `ChildStack::restore_from_saved()`
- ❌ Не добавляем макрос для NestedScreen
- ❌ Не пишем пример с диалогом/BottomSheet

### План

| # | Шаг | Инвазивность | Файлы | Тесты |
|---|-----|--------------|-------|-------|
| 1 | Удалить `ComponentContext2`, `StateKeeper`, `ChildStackManager`, `ComponentState`, `ComponentNode2` | 🔴 Высокая | 5 файлов удалить, 3 файла обновить (lib.rs, component_factory.rs, framework/lib.rs) | Тесты этих модулей удалятся; проверить, что остальные проходят |
| 2 | Упростить `ComponentContext`: убрать generic `NavEvent`/`DataCmd`, убрать `back_fallback`, оставить `BackDispatcher` + `finish_requested` | 🔴 Высокая | `component_context.rs`, `navigation_host.rs` | — |
| 3 | Добавить `ChildStack::on_back()` — цепочка `active.handle_back()` → `pop()` | 🟡 Низкая | `child_stack.rs` | Юнит: on_back с активным компонентом, on_back с пустым стеком |
| 4 | Переписать `NavigationHost` — тонкая обёртка над `ChildStack` + `on_back()` | 🟠 Средняя | `navigation_host.rs`, `app.rs` | — (интеграционный, showcase компилируется) |
| 5 | Обновить `lib.rs` / `framework/lib.rs` — убрать реэкспорты удалённых типов | 🟡 Низкая | `core/src/lib.rs`, `navigation/src/lib.rs`, `framework/src/lib.rs` | — |
| 6 | Запустить тесты, проверить showcase | 🟢 Проверка | Все | cargo test --workspace |

### Риски

| Риск | Вероятность | Митигация |
|------|-------------|-----------|
| `ChildStack::on_back()` ломает существующие тесты | Низкая | Добавляем новый метод, не меняем существующие |
| Удаление `ComponentContext` ломает `NavigationHost` | Высокая | Переписываем `NavigationHost` в том же шаге |
| `BackDispatcher` остаётся без использования | Не проблема | Оставляем для будущих диалогов, документировано |

---
