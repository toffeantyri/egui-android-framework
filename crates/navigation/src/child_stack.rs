//! ChildStack — стек дочерних компонентов с управлением жизненным циклом.
//!
//! Аналог `ChildStack` из Decompose. Хранит стек `(Config, ComponentNode)` пар,
//! где верхний элемент — активный (отображаемый) компонент.
//!
//! В отличие от старой версии, `ChildStack` не generic по типу компонента —
//! он хранит `Box<dyn ComponentNode>`, что позволяет складывать в один стек
//! компоненты разных типов сообщений (как в Decompose).
//!
//! # Параметры типа
//!
//! * `C` — конфигурация экрана (аналог Route/Config в Decompose).
//!   Должна быть `Clone + PartialEq` для идентификации.
//!
//! # Пример
//!
//! ```ignore
//! let mut stack = ChildStack::<Route>::new();
//! stack.push(Route::Home, Box::new(HomeScreen::new()));
//! stack.push(Route::Widgets, Box::new(WidgetsScreen::new()));
//!
//! if let Some(active) = stack.active() {
//!     active.render(ui, &dyn_dispatcher);
//! }
//! ```

use egui_android_core::ComponentNode;

/// Стек дочерних компонентов.
///
/// Хранит `(Config, Box<dyn ComponentNode>)` пары.
/// Generic только по конфигурации `C`.
pub struct ChildStack<C>
where
    C: Clone + PartialEq,
{
    items: Vec<ChildItem<C>>,
}

/// Элемент стека: конфигурация + компонент (type-erased).
struct ChildItem<C> {
    config: C,
    component: Box<dyn ComponentNode>,
}

impl<C> ChildStack<C>
where
    C: Clone + PartialEq,
{
    /// Создать пустой стек.
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Добавить компонент на вершину стека.
    pub fn push(&mut self, config: C, component: Box<dyn ComponentNode>) {
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
    /// Вызывает lifecycle на удаляемом компоненте.
    ///
    /// Возвращает `None`, если стек пуст.
    pub fn pop(&mut self) -> Option<(C, Box<dyn ComponentNode>)> {
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
    pub fn replace(&mut self, config: C, component: Box<dyn ComponentNode>) {
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
    pub fn bring_to_front(&mut self, config: C, component: Box<dyn ComponentNode>) {
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
    pub fn active(&self) -> Option<&dyn ComponentNode> {
        self.items.last().map(|item| &*item.component)
    }

    /// Получить мутабельную ссылку на активный компонент.
    pub fn active_mut(&mut self) -> Option<&mut dyn ComponentNode> {
        let idx = self.items.len().checked_sub(1)?;
        Some(&mut *self.items[idx].component)
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
        while let Some(removed) = self.items.pop() {
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

impl<C> Default for ChildStack<C>
where
    C: Clone + PartialEq,
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
    use egui_android_core::Component;
    use egui_android_core::LifecycleObserver;
    use egui_android_core::UiWrapper;
    use egui_android_runtime::Dispatcher;

    /// Тестовый компонент, логирующий вызовы lifecycle.
    #[derive(Default)]
    struct TestComp {
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

        fn render(&self, _ui: &mut UiWrapper, _dispatch: &Dispatcher<Self::Message>) {}

        fn handle(&mut self, _msg: Self::Message) {}

        fn state(&self) -> &Self::State {
            &()
        }
    }

    #[test]
    fn test_push_lifecycle() {
        let mut stack = ChildStack::<&'static str>::new();
        assert!(stack.is_empty());

        stack.push("screen_a", Box::new(TestComp::default()));
        assert_eq!(stack.len(), 1);
        assert_eq!(stack.active_config(), Some(&"screen_a"));
    }

    #[test]
    fn test_pop_lifecycle() {
        let mut stack = ChildStack::<&'static str>::new();
        stack.push("a", Box::new(TestComp::default()));
        stack.push("b", Box::new(TestComp::default()));

        assert_eq!(stack.len(), 2);
        let (config, mut comp) = stack.pop().unwrap();
        assert_eq!(config, "b");

        // Проверяем lifecycle через as_any_mut + downcast_ref
        let test_comp = comp
            .as_any_mut()
            .downcast_mut::<TestComp>()
            .expect("should downcast to TestComp");
        assert_eq!(
            test_comp.events,
            vec!["create", "start", "resume", "pause", "stop", "destroy"]
        );
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn test_replace_lifecycle() {
        let mut stack = ChildStack::<&'static str>::new();
        stack.push("a", Box::new(TestComp::default()));

        stack.replace("b", Box::new(TestComp::default()));

        assert_eq!(stack.len(), 1);
        assert_eq!(stack.active_config(), Some(&"b"));
    }

    #[test]
    fn test_bring_to_front_new_config() {
        let mut stack = ChildStack::<&'static str>::new();
        stack.push("a", Box::new(TestComp::default()));

        // Нету — заменяет верхний
        stack.bring_to_front("b", Box::new(TestComp::default()));
        assert_eq!(stack.active_config(), Some(&"b"));
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn test_clear_destroys_all() {
        let mut stack = ChildStack::<&'static str>::new();
        stack.push("a", Box::new(TestComp::default()));
        stack.push("b", Box::new(TestComp::default()));

        stack.clear();
        assert!(stack.is_empty());
    }

    #[test]
    fn test_pop_empty_stack() {
        let mut stack = ChildStack::<&'static str>::new();
        assert!(stack.pop().is_none());
    }

    #[test]
    fn test_active_on_empty_stack() {
        let stack = ChildStack::<&'static str>::new();
        assert!(stack.active().is_none());
    }

    #[test]
    fn test_active_mut_on_empty_stack() {
        let mut stack = ChildStack::<&'static str>::new();
        assert!(stack.active_mut().is_none());
    }
}
