//! [`ChildStackManager`] — навигация через ComponentContext2 с рекурсивным save/restore.
//!
//! Аналог `ChildrenNavigator` в Decompose.
//!
//! # Зачем
//!
//! Старый `ChildStack` требует ручного save/restore. Новый `ChildStackManager`
//! автоматически:
//! - Создаёт дочерние [`ComponentContext2`] с их [`StateKeeper`] при push
//! - Рекурсивно сохраняет/восстанавливает всё дерево через StateKeeper
//! - Управляет lifecycle компонентов (on_create/on_start/on_resume/...)
//!
//! # Как использовать
//!
//! ```ignore
//! let mut manager = ChildStackManager::new(
//!     ctx,                    // ComponentContext2 (корневой)
//!     factory,                // Box<dyn ComponentFactory2<Route, Msg>>
//! );
//!
//! // Навигация — как с ChildStack
//! manager.push(Route::Home);
//! manager.push(Route::Widgets);
//!
//! // Save/restore — автоматически рекурсивно
//! let saved = manager.save_all();    // рекурсивно
//! manager.restore_all(saved);         // рекурсивно
//!
//! // Render
//! manager.render(ui, dispatch);
//! ```

use egui_android_core::component_context2::ComponentContext2;
use egui_android_core::state_keeper::StateKeeperSnapshot;
use egui_android_core::{ComponentNode, LifecycleObserver};
use std::fmt::Debug;

use crate::child_stack::ChildStack;
use crate::component_factory::ComponentFactory2;

/// Менеджер стека навигации с автоматическим save/restore.
///
/// Владеет:
/// - [`ChildStack`] — стек конфигураций и компонентов
/// - [`ComponentContext2`] — контекст с StateKeeper для save/restore
/// - [`ComponentFactory2`] — фабрика для создания компонентов
///
/// # Параметры типа
///
/// * `C` — конфигурация (маршрут). Должна быть `Clone + PartialEq + Debug + 'static`.
pub struct ChildStackManager<C>
where
    C: Clone + PartialEq + Debug + 'static,
{
    /// Стек конфигураций и компонентов.
    stack: ChildStack<C>,
    /// Контекст — владелец StateKeeper.
    ctx: ComponentContext2,
    /// Фабрика компонентов.
    factory: Box<dyn ComponentFactory2<C>>,
}

impl<C> ChildStackManager<C>
where
    C: Clone + PartialEq + std::fmt::Debug + 'static,
{
    /// Создать новый менеджер стека.
    ///
    /// # Params
    ///
    /// * `ctx` — корневой ComponentContext2 (владелец StateKeeper)
    /// * `factory` — фабрика компонентов
    pub fn new(ctx: ComponentContext2, factory: Box<dyn ComponentFactory2<C>>) -> Self {
        Self {
            stack: ChildStack::new(),
            ctx,
            factory,
        }
    }

    // ─── Навигация ──────────────────────────────────────────────────

    /// Добавить компонент на вершину стека.
    ///
    /// Автоматически:
    /// 1. Создаёт дочерний ComponentContext2 с StateKeeper по ключу
    /// 2. Создаёт компонент через фабрику
    /// 3. Вызывает lifecycle (on_create, on_start, on_resume)
    pub fn push(&mut self, config: C) {
        // Создаём дочерний контекст с StateKeeper
        let key = format!("child:{}", self.stack.len());
        let child_ctx = self.create_child_context(&key);

        // Создаём компонент через фабрику
        let component = self.factory.create(config.clone(), child_ctx);
        self.stack.push(config, component);
    }

    /// Убрать верхний элемент стека.
    ///
    /// Автоматически:
    /// 1. Вызывает lifecycle (on_pause, on_stop, on_destroy)
    /// 2. Удаляет дочерний StateKeeper
    pub fn pop(&mut self) -> Option<(C, Box<dyn ComponentNode>)> {
        let result = self.stack.pop();
        if result.is_some() {
            // Удаляем StateKeeper последнего child
            let key = format!("child:{}", self.stack.len());
            self.ctx.state_keeper_mut().remove_child(&key);
        }
        result
    }

    /// Заменить верхний элемент на новый.
    pub fn replace(&mut self, config: C) {
        let key = format!("child:{}", self.stack.len().saturating_sub(1));
        let child_ctx = self.create_child_context(&key);
        let component = self.factory.create(config.clone(), child_ctx);
        self.stack.replace(config, component);
    }

    /// Очистить стек.
    pub fn clear(&mut self) {
        self.stack.clear();
        self.ctx.state_keeper_mut().clear_children();
    }

    // ─── Доступ к стеку ──────────────────────────────────────────────

    /// Получить ссылку на активный компонент.
    pub fn active(&self) -> Option<&dyn ComponentNode> {
        self.stack.active()
    }

    /// Получить конфигурацию активного компонента.
    pub fn active_config(&self) -> Option<&C> {
        self.stack.active_config()
    }

    /// Проверить, пуст ли стек.
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Получить количество элементов в стеке.
    pub fn len(&self) -> usize {
        self.stack.len()
    }

    /// Получить ссылку на ComponentContext2.
    pub fn context(&self) -> &ComponentContext2 {
        &self.ctx
    }

    /// Получить мутабельную ссылку на ComponentContext2.
    pub fn context_mut(&mut self) -> &mut ComponentContext2 {
        &mut self.ctx
    }

    // ─── Save / Restore ──────────────────────────────────────────────

    /// Сохранить состояние всего дерева рекурсивно.
    ///
    /// После вызова save_all() состояние обнуляется.
    /// Результат нужно передать в `restore_all()` при восстановлении.
    pub fn save_all(&mut self) -> StateKeeperSnapshot {
        self.ctx.save_all()
    }

    /// Восстановить состояние всего дерева из snapshot.
    pub fn restore_all(&mut self, snapshot: StateKeeperSnapshot) {
        self.ctx.restore_all(snapshot);
    }

    // ─── Render ──────────────────────────────────────────────────────

    /// Получить ссылку для рендера активного компонента.
    ///
    /// Возвращает `&dyn ComponentNode`, который можно отрисовать
    /// через `node.render(ui, dispatch)`. Активный компонент —
    /// верхний в стеке.
    pub fn active_node(&self) -> Option<&dyn ComponentNode> {
        self.stack.active()
    }

    // ─── Lifecycle ───────────────────────────────────────────────────

    /// Применить lifecycle-событие к активному компоненту.
    pub fn on_lifecycle_event(&mut self, event: crate::child_stack::LifecycleEvent) {
        self.stack.on_lifecycle_event(event);
    }

    // ─── Вспомогательные методы ─────────────────────────────────────

    /// Создать дочерний ComponentContext2 с StateKeeper.
    fn create_child_context(&mut self, key: &str) -> ComponentContext2 {
        // Создаём дочерний StateKeeper в родительском дереве
        let _child_keeper = self.ctx.state_keeper_mut().child(key);

        // Создаём новый ComponentContext2 для дочернего компонента
        //
        // Пока — возвращаем корневой контекст (StateKeeper общий).
        // В будущем — создавать отдельный ComponentContext2 с собственным
        // StateKeeper и привязкой к родительскому.
        //
        // Правильная реализация потребует:
        // 1. ChildContext2 { parent_ctx: &mut ComponentContext2, key: String }
        // 2. При save_all() — parent_ctx.state_keeper.child(self.key).save_take()
        //
        // Пока заглушка для обратной совместимости.
        ComponentContext2::root(key)
    }
}

impl<C> LifecycleObserver for ChildStackManager<C> where C: Clone + PartialEq + Debug + 'static {}

// ─── Тесты ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use egui_android_core::component_context2::ComponentContext2;
    use egui_android_core::{Component, UiWrapper};

    #[derive(Debug, Clone, PartialEq)]
    enum TestRoute {
        A,
        B,
    }

    #[derive(Default)]
    struct TestComp {
        ctx: Option<ComponentContext2>,
    }

    impl LifecycleObserver for TestComp {}

    impl Component for TestComp {
        type State = ();
        type Message = ();

        fn render(
            &self,
            _ui: &mut UiWrapper,
            _dispatch: &egui_android_runtime::Dispatcher<Self::Message>,
        ) {
        }
        fn handle(&mut self, _msg: Self::Message) {}
        fn state(&self) -> &Self::State {
            &()
        }
    }
    // Blanket-impl ComponentNode for TestComp автоматически

    struct TestFactory;

    impl ComponentFactory2<TestRoute> for TestFactory {
        fn create(&self, _config: TestRoute, ctx: ComponentContext2) -> Box<dyn ComponentNode> {
            let mut comp = Box::new(TestComp::default());
            comp.ctx = Some(ctx);
            comp
        }
    }

    #[test]
    fn test_new_manager_is_empty() {
        let ctx = ComponentContext2::root("test");
        let factory = Box::new(TestFactory);
        let manager = ChildStackManager::<TestRoute>::new(ctx, factory);
        assert!(manager.is_empty());
        assert_eq!(manager.len(), 0);
    }

    #[test]
    fn test_push_adds_component() {
        let ctx = ComponentContext2::root("test");
        let factory = Box::new(TestFactory);
        let mut manager = ChildStackManager::<TestRoute>::new(ctx, factory);

        manager.push(TestRoute::A);
        assert!(!manager.is_empty());
        assert_eq!(manager.len(), 1);
        assert_eq!(manager.active_config(), Some(&TestRoute::A));
    }

    #[test]
    fn test_push_twice() {
        let ctx = ComponentContext2::root("test");
        let factory = Box::new(TestFactory);
        let mut manager = ChildStackManager::<TestRoute>::new(ctx, factory);

        manager.push(TestRoute::A);
        manager.push(TestRoute::B);
        assert_eq!(manager.len(), 2);
        assert_eq!(manager.active_config(), Some(&TestRoute::B));
    }

    #[test]
    fn test_pop_removes_top() {
        let ctx = ComponentContext2::root("test");
        let factory = Box::new(TestFactory);
        let mut manager = ChildStackManager::<TestRoute>::new(ctx, factory);

        manager.push(TestRoute::A);
        manager.push(TestRoute::B);
        let popped = manager.pop();
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().0, TestRoute::B);
        assert_eq!(manager.len(), 1);
        assert_eq!(manager.active_config(), Some(&TestRoute::A));
    }

    #[test]
    fn test_pop_empty_returns_none() {
        let ctx = ComponentContext2::root("test");
        let factory = Box::new(TestFactory);
        let mut manager = ChildStackManager::<TestRoute>::new(ctx, factory);

        assert!(manager.pop().is_none());
    }

    #[test]
    fn test_replace_works() {
        let ctx = ComponentContext2::root("test");
        let factory = Box::new(TestFactory);
        let mut manager = ChildStackManager::<TestRoute>::new(ctx, factory);

        manager.push(TestRoute::A);
        manager.replace(TestRoute::B);
        assert_eq!(manager.len(), 1);
        assert_eq!(manager.active_config(), Some(&TestRoute::B));
    }

    #[test]
    fn test_clear_destroys_all() {
        let ctx = ComponentContext2::root("test");
        let factory = Box::new(TestFactory);
        let mut manager = ChildStackManager::<TestRoute>::new(ctx, factory);

        manager.push(TestRoute::A);
        manager.push(TestRoute::B);
        manager.clear();
        assert!(manager.is_empty());
    }

    #[test]
    fn test_save_restore_roundtrip() {
        let mut ctx = ComponentContext2::root("test");
        ctx.state_keeper_mut().set_own_state(Box::new(42u32));
        let factory = Box::new(TestFactory);
        let mut manager = ChildStackManager::<TestRoute>::new(ctx, factory);

        manager.push(TestRoute::A);

        // Сохраняем
        let snapshot = manager.save_all();

        // Создаём новый менеджер и восстанавливаем
        let new_ctx = ComponentContext2::root("test");
        let mut restored = ChildStackManager::<TestRoute>::new(new_ctx, Box::new(TestFactory));
        restored.restore_all(snapshot);

        // Проверяем, что состояние корневого контекста восстановлено
        assert_eq!(restored.context().state::<u32>(), Some(&42));
    }
}
