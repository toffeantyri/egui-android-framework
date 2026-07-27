# Проблемы

## 2. Type erasure — runtime ошибка вместо compile-time

```
Проблема: Type erasure через Box<dyn Any> теряет типобезопасность

Файл: crates/core/src/component_node.rs

Суть:
  fn handle_dyn(&mut self, msg: Box<dyn std::any::Any + Send>) {
      if let Ok(typed) = msg.downcast::<T::Message>() {
          crate::Component::handle(self, *typed);
      } else {
          log::error!("ожидался {}, получен неизвестный", ...);
      }
  }

  Неправильный тип сообщения = тихий log::error в рантайме,
  а не ошибка компиляции.

  DynDispatcher (crates/runtime/src/dyn_dispatcher.rs) упаковывает
  M в Box<dyn Any + Send> — вся типобезопасность теряется на
  границе View → Component.

Влияние:
  - Ошибки маршрутизации сообщений обнаруживаются только в рантайме
  - Нет compile-time гарантии, что View шлёт правильный тип

Затрагиваемые слои: core, runtime
Риск: средний — требует изменения публичного API
```

## 4. Гибридные Constraints — два источника правды [ВЫПОЛНЕНО]

```
Проблема: Constraints хранились в двух местах одновременно

Файл: crates/core/src/ui_wrapper.rs

Решение: Вариант A — только Context::data(), поле убрано.
Единственный источник правды. См. Задача 4.
```

## 5. remember() мутирует состояние внутри рендера

```
Проблема: remember() нарушает декларацию «store.update() — единственная точка мутации»

Файл: crates/ui/src/remember.rs

Суть:
  pub fn set(&self, new_value: T) {
      *self.value.write().expect("...") = new_value.clone();
      self.persist(&new_value);  // ← мутация ctx.data_mut внутри рендера
  }

  on_click_with() делает то же самое — вызывает closure в момент рендера.

  arch.md декларирует: «store.update() — единственная точка изменения состояния».
  Но remember() и on_click_with() мутируют состояние напрямую.

Влияние:
  - Архитектурное противоречие между декларацией и реализацией
  - Неочевидно для разработчика, какие данные где мутируются

Затрагиваемые слои: ui, документация (arch.md)
Риск: нулевой при документировании, высокий при изменении кода
```

## 9. Blanket-impl конфликт [ВЫПОЛНЕНО]

```
Проблема: Rust запрещает два blanket-impl для одного трейта.
Решение: blanket-impl удалён, заменён на #[derive(ComponentNode)].
PersistentComponent<T> удалён. См. Задача 5.
```

---

# Задачи (решения)

## Задача 3: Документировать гибридную MVI + Local State модель [ВЫПОЛНЕНО]

```
Задача: Задокументировать гибридную модель — MVI для бизнес-данных + локальное UI-состояние

Что сделано:
  1. В arch.md добавлен раздел «Гибридная модель: MVI + Local State» с диаграммой
  2. В guide.md раздел «Упрощённая MVI-модель» → «Гибридная модель»
  3. В arch.md обновлён контракт (UI не изменяет бизнес-State)
  4. Добавлена таблица-аналогия с Jetpack Compose

Проверка: ревью документации

Затрагиваемые слои: документация
```

## Задача 4: Убрать гибридное хранение Constraints [ВЫПОЛНЕНО]

```
Задача: Оставить один источник правды для Constraints

Что сделано (Вариант A):
  1. Из UiWrapper убрано поле Constraints из Borrowed/Owned
  2. constraints() читает из Context::data()
  3. set_constraints() пишет в Context::data()
  4. Удалён constraints_mut()
  5. Обновлён Modifier::apply_recursive (убрано *ui.constraints())
  6. Обновлены тесты layout_tests

Проверка: cargo test --workspace

Затрагиваемые слои: core, ui
```

## Задача 5: Макрос #[derive(ComponentNode)] для устранения обёртки [ВЫПОЛНЕНО]

```
Задача: Убрать необходимость вручную оборачивать в PersistentComponent

Что сделано:
  1. Добавлен proc-macro #[derive(ComponentNode)] с #[component_message(MsgType)]
  2. Blanket-impl ComponentNode for T: Component удалён из component_node.rs
  3. PersistentComponent<T> удалён из persistent_state.rs и core/lib.rs
  4. У всех экранов showcase и counter добавлен #[derive(ComponentNode)]
  5. StateScreen — ручной impl (кастомный take_back_request)
  6. BackCustomScreen — ручной impl (кастомный handle_back)
  7. NestedScreen — ручной impl (кастомный handle_back, handle_dyn)
  8. Из фабрики убрана обёртка PersistentComponent::new(...)

Проверка: cargo test --workspace

Затрагиваемые слои: macros, core, navigation, examples
```

## Задача 6: Единая система обработки Back [ВЫПОЛНЕНО]

```
Задача: Объединить системный Back и кнопку "← Назад" в единый механизм

Что сделано:
  1. В ComponentNode добавлен метод take_back_request() (default = false)
  2. В NavigationHost добавлен check_back_request() — проверяет флаг после handle_dyn
  3. В app.rs после handle_dyn вызывается check_back_request()
  4. StateScreen: ручной impl ComponentNode + back_requested + кнопка Back
  5. BackCustomScreen: ручной impl ComponentNode с кастомным handle_back()
  6. Обновлена документация в arch.md и guide.md

Корнер-кейсы:
  - handle_back() -> true: перехват без pop (NestedScreen, BackCustomScreen)
  - take_back_request() -> true: кастомная логика + pop (StateScreen)
  - handle_back() -> false + take_back_request() -> false: обычный pop или finish

Затрагиваемые слои: core, showcase (navigation_host, app, screens)
```

## Задача 8: Enum-based dispatch для compile-time safety

```
Задача: Заменить Box<dyn Any> downcast на типизированный enum

Что сделать:
  1. Генерировать enum сообщений макросом из Route:

     // Генерируется из enum Route { State, Widgets, ... }
     enum ScreenMsg {
         StateScreen(state_screen::Msg),
         WidgetsScreen(widgets::Msg),
         // ...
     }

  2. ComponentNode получает типизированный handle:
     fn handle_enum(&mut self, msg: ScreenMsg) {
         match msg {
             ScreenMsg::StateScreen(m) => self.handle(m),
             // ...
         }
     }

  3. DynDispatcher остаётся для динамических случаев (плагины),
     но основной путь — типизированный enum

  4. Ошибка в типе сообщения = compile error

Проверка:
  cargo check --workspace
  cargo test --workspace

Затрагиваемые слои: core, runtime, macros, navigation
Оценка: 2 недели
```

## Задача 9: Desktop/Web платформы

```
Задача: Реализовать кросс-платформенность через platform-desktop

Что сделать:
  1. Создать crates/platform-desktop/:
     - winit event loop
     - glow renderer (уже используется в platform-android)
     - run<A: Application>() с тем же контрактом

  2. Выделить общий код из platform-android в platform:
     - RunState::tick() → общий для всех платформ
     - GraphicsPipeline → общий (glow)
     - Input processing → адаптер под winit events

  3. platform-android остаётся Android-специфичным:
     - EGL, GameActivity, JNI, IME

  4. Проверить, что Application trait не содержит Android-специфики

Проверка:
  cargo check --workspace
  cargo run -p example-counter  # desktop
  cargo ndk build -p example-counter  # android

Затрагиваемые слои: platform, platform-android, новый platform-desktop
Оценка: 2 месяца
```


---
