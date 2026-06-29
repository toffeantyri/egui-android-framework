//! ChildStack — стек дочерних компонентов с управлением жизненным циклом.
//!
//! Аналог `ChildStack` из Decompose. Хранит стек `(Config, Component)` пар,
//! где верхний элемент — активный (отображаемый) компонент.
//!
//! При push/pop/replace вызываются lifecycle-методы компонентов
//! (on_create, on_destroy).

use crate::component::Component;

/// Стек дочерних компонентов.
///
/// # Параметры типа
///
/// * `C` — конфигурация экрана (аналог Route/Config в Decompose).
///   Должна быть `Clone + PartialEq` для идентификации.
/// * `Comp` — тип компонента (обычно `Box<dyn Component>`).
pub struct ChildStack<C, Comp>
where
    C: Clone + PartialEq,
    Comp: Component,
{
    items: Vec<ChildItem<C, Comp>>,
}

/// Элемент стека: конфигурация + компонент.
struct ChildItem<C, Comp> {
    config: C,
    component: Comp,
}

impl<C, Comp> ChildStack<C, Comp>
where
    C: Clone + PartialEq,
    Comp: Component,
{
    /// Создать пустой стек.
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Добавить компонент на вершину стека.
    ///
    /// Вызывает `on_create()` на новом компоненте.
    /// Если стек не пуст — вызывает `on_pause()` на предыдущем активном.
    pub fn push(&mut self, config: C, component: Comp) {
        if let Some(active) = self.items.last_mut() {
            active.component.on_pause();
        }
        let mut comp = component;
        comp.on_create();
        comp.on_start();
        comp.on_resume();
        self.items.push(ChildItem {
            config,
            component: comp,
        });
    }

    /// Убрать верхний элемент стека.
    ///
    /// Вызывает `on_destroy()` на удаляемом компоненте.
    /// Если стек не пуст — вызывает `on_resume()` на новом верхнем.
    ///
    /// Возвращает `None`, если стек пуст.
    pub fn pop(&mut self) -> Option<(C, Comp)> {
        let removed = self.items.pop()?;
        let mut comp = removed.component;
        comp.on_pause();
        comp.on_stop();
        comp.on_destroy();

        if let Some(active) = self.items.last_mut() {
            active.component.on_resume();
        }

        Some((removed.config, comp))
    }

    /// Заменить верхний элемент на новый (pop + push без анимации).
    ///
    /// Если стек пуст — равносильно `push()`.
    pub fn replace(&mut self, config: C, component: Comp) {
        if self.items.is_empty() {
            self.push(config, component);
            return;
        }

        // destroy старого
        let mut old = self.items.pop().unwrap();
        old.component.on_pause();
        old.component.on_stop();
        old.component.on_destroy();

        // create нового
        let mut comp = component;
        comp.on_create();
        comp.on_start();
        comp.on_resume();
        self.items.push(ChildItem {
            config,
            component: comp,
        });
    }

    /// Переместить стек в указанное состояние.
    ///
    /// Все элементы выше последнего вхождения `config` удаляются.
    /// Если `config` нет в стеке — равносильно `replace()`.
    ///
    /// Аналог `bringToFront` в Decompose.
    pub fn bring_to_front(&mut self, config: C, component: Comp) {
        // Ищем позицию с такой же конфигурацией
        if let Some(pos) = self.items.iter().position(|item| item.config == config) {
            // Удаляем всё, что выше этой позиции
            while self.items.len() > pos + 1 {
                let removed = self.items.pop().unwrap();
                let mut comp = removed.component;
                comp.on_pause();
                comp.on_stop();
                comp.on_destroy();
            }
            // Текущий верхний (pos) уже на месте — не трогаем
        } else {
            // Конфига нет — заменяем верхний
            self.replace(config, component);
        }
    }

    /// Получить ссылку на активный (верхний) компонент.
    pub fn active(&self) -> Option<&Comp> {
        self.items.last().map(|item| &item.component)
    }

    /// Получить мутабельную ссылку на активный компонент.
    pub fn active_mut(&mut self) -> Option<&mut Comp> {
        let idx = self.items.len().checked_sub(1)?;
        Some(&mut self.items[idx].component)
    }

    /// Получить конфигурацию активного компонента.
    pub fn active_config(&self) -> Option<&C> {
        self.items.last().map(|item| &item.config)
    }

    /// Проверить, пуст ли стек.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Получить количество элементов в стеке.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Очистить стек (уничтожить все компоненты).
    pub fn clear(&mut self) {
        while !self.items.is_empty() {
            let removed = self.items.pop().unwrap();
            let mut comp = removed.component;
            comp.on_pause();
            comp.on_stop();
            comp.on_destroy();
        }
    }

    /// Применить lifecycle-метод ко всем элементам стека.
    pub fn on_lifecycle_event(&mut self, event: LifecycleEvent) {
        match event {
            LifecycleEvent::Resume => {
                if let Some(active) = self.items.last_mut() {
                    active.component.on_resume();
                }
            }
            LifecycleEvent::Pause => {
                if let Some(active) = self.items.last_mut() {
                    active.component.on_pause();
                }
            }
        }
    }
}

impl<C, Comp> Default for ChildStack<C, Comp>
where
    C: Clone + PartialEq,
    Comp: Component,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Событие жизненного цикла для ChildStack.
///
/// Пробрасывается на активный (верхний) компонент стека.
pub enum LifecycleEvent {
    /// Вызвать `on_resume()` на активном компоненте.
    Resume,
    /// Вызвать `on_pause()` на активном компоненте.
    Pause,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LifecycleObserver;

    /// Тестовый компонент, логирующий вызовы lifecycle.
    #[derive(Default)]
    struct TestComp {
        pub id: &'static str,
        pub events: Vec<&'static str>,
    }

    impl LifecycleObserver for TestComp {
        fn on_create(&mut self) {
            self.events.push("create");
        }
        fn on_start(&mut self) {
            self.events.push("start");
        }
        fn on_resume(&mut self) {
            self.events.push("resume");
        }
        fn on_pause(&mut self) {
            self.events.push("pause");
        }
        fn on_stop(&mut self) {
            self.events.push("stop");
        }
        fn on_destroy(&mut self) {
            self.events.push("destroy");
        }
    }

    impl Component for TestComp {
        type State = ();
        type Message = ();

        fn render(
            &self,
            _ui: &mut egui::Ui,
            _dispatch: &crate::dispatcher::Dispatcher<Self::Message>,
        ) {
        }

        fn handle(&mut self, _msg: Self::Message) {}

        fn state(&self) -> &Self::State {
            &()
        }
    }

    #[test]
    fn test_push_lifecycle() {
        let mut stack = ChildStack::<&'static str, TestComp>::new();
        assert!(stack.is_empty());

        stack.push(
            "screen_a",
            TestComp {
                id: "a",
                events: Vec::new(),
            },
        );
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.active_config(), Some(&"screen_a"));

        let active = stack.active().unwrap();
        assert_eq!(active.events, vec!["create", "start", "resume"]);
    }

    #[test]
    fn test_pop_lifecycle() {
        let mut stack = ChildStack::<&'static str, TestComp>::new();
        stack.push(
            "a",
            TestComp {
                id: "a",
                events: Vec::new(),
            },
        );
        stack.push(
            "b",
            TestComp {
                id: "b",
                events: Vec::new(),
            },
        );

        assert_eq!(stack.len(), 2);
        let (config, comp) = stack.pop().unwrap();
        assert_eq!(config, "b");
        // Компонент B прошёл полный цикл: create → start → resume → pause → stop → destroy
        assert_eq!(
            comp.events,
            vec!["create", "start", "resume", "pause", "stop", "destroy"]
        );
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn test_replace_lifecycle() {
        let mut stack = ChildStack::<&'static str, TestComp>::new();
        stack.push(
            "a",
            TestComp {
                id: "a",
                events: Vec::new(),
            },
        );

        stack.replace(
            "b",
            TestComp {
                id: "b",
                events: Vec::new(),
            },
        );

        assert_eq!(stack.len(), 1);
        assert_eq!(stack.active_config(), Some(&"b"));

        let old_events = stack.active().unwrap().events.clone();
        assert_eq!(old_events, vec!["create", "start", "resume"]);
    }

    #[test]
    fn test_bring_to_front_new_config() {
        let mut stack = ChildStack::<&'static str, TestComp>::new();
        stack.push(
            "a",
            TestComp {
                id: "a",
                events: Vec::new(),
            },
        );

        // Нету — заменяет верхний
        stack.bring_to_front(
            "b",
            TestComp {
                id: "b",
                events: Vec::new(),
            },
        );
        assert_eq!(stack.active_config(), Some(&"b"));
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn test_clear_destroys_all() {
        let mut stack = ChildStack::<&'static str, TestComp>::new();
        stack.push(
            "a",
            TestComp {
                id: "a",
                events: Vec::new(),
            },
        );
        stack.push(
            "b",
            TestComp {
                id: "b",
                events: Vec::new(),
            },
        );

        stack.clear();
        assert!(stack.is_empty());
    }

    #[test]
    fn test_pop_empty_stack() {
        let mut stack = ChildStack::<&'static str, TestComp>::new();
        assert!(stack.pop().is_none());
    }

    #[test]
    fn test_active_on_empty_stack() {
        let mut stack = ChildStack::<&'static str, TestComp>::new();
        assert!(stack.active().is_none());
        assert!(stack.active_mut().is_none());
    }
}
