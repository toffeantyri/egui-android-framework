Это — актуальная дорожная карта для выхода фреймворка на продакшн‑уровень.

---

🟥 Приоритет 1 — критические архитектурные проблемы (остались после исправления Waker)

🟥 1. egui-android-platform-android

Проблема: глобальное состояние + утечки JNI/EGL
Исправление Waker не затрагивает эту область — она остаётся главной архитектурной дырой.

Что не так:
- Глобальные Atomic* и OnceLock (INSETLEFT, SYSTEMTHEME, GLOBALVM, SYSTEMBARS_COLORS).  
- Backend API всё ещё раскрывает детали платформы.  
- Два источника истины: часть состояния в backend, часть в egui::Context::data().

Что делать:
- Перенести ВСЁ состояние в backend instance.  
- Удалить глобальные статики.  
- Убрать JNI/EGL из публичного API backend.  
- PlatformState хранить только в backend → UI получает всё через FrameInput.

Приоритет: 🟥🟥🟥  
Крейт: egui-android-platform-android

---

🟥 2. egui-android-navigation

Проблема: нет полноценного save/restore
Исправление Waker не влияет на навигацию — она остаётся слабым звеном.

Что не так:
- ChildStack::restore() зависит от порядка элементов.  
- Нет сериализации конфигураций.  
- Нет типобезопасного состояния компонентов.  
- Нет Router/Configuration слоя.

Что делать:
- Ввести ComponentState trait.  
- ChildStack::save() → Vec<(C, ComponentState)>.  
- ChildStack::restore() → восстановление по конфигурациям.  
- Добавить Router + Configuration (как в Decompose).

Приоритет: 🟥🟥🟥  
Крейт: egui-android-navigation

---

🟥 3. egui-android-core

Проблема: type‑erasure через dyn Any
Исправление Waker не затрагивает messaging — проблема остаётся.

Что не так:
- Сообщения хранятся как Box<dyn Any + Send>.  
- Ошибки типов проявляются только в runtime.  
- Невозможно статически проверить корректность сообщений.

Что делать:
- Ввести trait‑bounds: Msg: Clone + Debug + Send + 'static.  
- Убрать type‑erasure из DynDispatcher.  
- Ввести типобезопасный MessageEnvelope.

Приоритет: 🟥🟥  
Крейт: egui-android-core

---

🟧 Приоритет 2 — серьёзные проблемы (не критические, но мешают развитию)

🟧 4. egui-android-platform-android

Проблема: backend монолитный (EGL/IME/Input/SystemBars смешаны)
Исправление Waker не уменьшает сложность backend.

Что не так:
- Backend слишком большой.  
- GlBackend и NativeBackend дублируют код.  
- run.rs огромный и не тестируемый.

Что делать:
- Разделить backend на модули:  
  - GraphicsBackend  
  - InputBackend  
  - ImeBackend  
  - SystemBarsBackend  
- Разбить run.rs на подмодули.

Приоритет: 🟧🟧  
Крейт: egui-android-platform-android

---

🟧 5. egui-android-ui

Проблема: перегруженный Modifier API
Исправление Waker не влияет на UI‑слой — он всё ещё слишком тяжёлый.

Что не так:
- Слишком много редких модификаторов.  
- Анимации смешаны с layout‑модификаторами.  
- Нарушение KISS/YAGNI.

Что делать:
- Вынести редкие модификаторы в ui-extras.  
- Анимации вынести в ui-animation.  
- Оставить минимальный набор базовых модификаторов.

Приоритет: 🟧  
Крейт: egui-android-ui

---

🟧 6. egui-android-core

Проблема: ComponentContext слишком сложный
Исправление Waker не затрагивает этот слой.

Что не так:
- Смешение навигации, data layer, back‑dispatcher.  
- Три generic‑параметра → сложно объяснить.

Что делать:
- Разделить на:  
  - NavigationContext  
  - StateContext  
  - BackContext  
  - DataContext

Приоритет: 🟧  
Крейт: egui-android-core

---

🟨 Приоритет 3 — второстепенные проблемы (косметика)

🟨 7. egui-android-platform

Проблема: PlatformConfig смешивает ответственность
Исправление Waker не влияет.

Что не так:
- target_fps относится к runtime.  
- log_tag относится к runtime.

Что делать:
- PlatformConfig → только платформенные параметры.  
- RuntimeConfig → FPS, логирование.

Приоритет: 🟨  
Крейт: egui-android-platform

---

🟨 8. egui-android-ui

Проблема: хрупкие layout‑тесты
Исправление Waker не влияет.

Что не так:
- Тесты завязаны на внутренние детали measure/layout.

Что делать:
- Тестировать только публичный контракт.

Приоритет: 🟨  
Крейт: egui-android-ui

---

🟨 9. egui-android-platform-android

Проблема: гибридное хранение PlatformState
Исправление Waker не влияет.

Что не так:
- Состояние хранится в backend и в egui::Context::data().

Что делать:
- Хранить только в backend.

Приоритет: 🟨  
Крейт: egui-android-platform-android

---

🟩 Приоритет 4 — низкий

🟩 10. egui-android-runtime

Проблема: DataLayerHandle заглушка
Исправление Waker не влияет.

Что делать:
- Ввести полноценный data layer или удалить заглушку.

---

🟩 11. egui-android-platform-android

Проблема: патч egui
Исправление Waker не влияет.

Что делать:
- Удалить патч, когда upstream исправит баги.

---

📌 Итоговая таблица по крейтам (обновлённая)

| Крейт | Приоритет | Проблемы |
|------|-----------|----------|
| platform-android | 🟥🟥🟥 | глобальные статики, JNI/EGL утечки, монолит backend, run.rs слишком большой, двойное хранение состояния |
| platform | 🟨 | PlatformConfig смешивает ответственность |
| runtime | 🟩 | DataLayerHandle заглушка |
| core | 🟥🟥 | dyn Any, сложный ComponentContext |
| navigation | 🟥🟥🟥 | слабый save/restore, нет Router |
| ui | 🟧 | перегруженный Modifier, хрупкие тесты |
| framework | 🟩 | косметика |

---
