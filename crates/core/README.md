# egui-android-core

**MVI-примитивы для egui-приложений на Android.**

Крейт содержит базовые типы и трейты для построения UI-компонентов
с однонаправленным потоком данных: Component, ViewFn, Widget, LifecycleObserver,
а также инфраструктурные элементы: UiWrapper (обёртка над egui::Ui с Constraints),
BackDispatcher (обработка кнопки Back), Constraints (Compose-like ограничения).

[![crates.io](https://img.shields.io/crates/v/egui-android-core)](https://crates.io/crates/egui-android-core)

## Состав

### `Component` — узел дерева навигации
- Хранит snapshot состояния + ссылку на Store
- `render(ui, &dispatcher)` — делегирует View-функции
- `handle(msg)` — обрабатывает сообщение (команда в data layer)
- `sync_from_store()` — обновляет snapshot из Store
- Наследует `LifecycleObserver`

### `Widget<M>` — трейт для всех виджетов
- `render(&self, ui: &mut UiWrapper, dispatch)` — рендерит виджет
- Принимает `&mut UiWrapper` — даёт доступ к Constraints (аналог Compose BoxWithConstraints)
- Generic `M` — тип сообщения для диспатча

### `UiWrapper` — обёртка над egui::Ui
- Хранит `Constraints` в поле + в `Context::data()` (переживает `Frame::show`)
- `allocate_space(size)` — alloc с учётом constraints
- `Deref<Target = egui::Ui>` — полная совместимость
- Owned/Borrowed варианты для child_ui

### `Constraints` — Compose-like ограничения размера
- `min_width`, `max_width`, `min_height`, `max_height`
- `exact(w, h)` / `ranged(min_w, max_w, min_h, max_h)` / `unconstrained()`

### `LifecycleObserver`
- `on_create / on_start / on_resume / on_pause / on_stop / on_destroy`
- Все методы имеют пустую реализацию по умолчанию

### `BackDispatcher`
- Центральный менеджер кнопки Back
- Регистрация callback'ов с приоритетами
- Обработка: диалоги → кастомная логика → ChildStack pop → завершение

## Когда использовать

Подключайте `egui-android-core`, если вы:
- пишете свой виджет/контейнер (нужен Widget трейт, UiWrapper, Constraints)
- работаете с навигацией и жизненным циклом
- реализуете кастомную обработку Back

Для готовых виджетов используйте `egui-android-ui` (реэкспортится через core).
