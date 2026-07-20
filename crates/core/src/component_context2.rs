//! [`ComponentContext2`] — новый контекст компонента с StateKeeper.
//!
//! В отличие от старого [`ComponentContext`], который generic по
//! (NavEvent, DataCmd, State) и предоставляет каналы, новый контекст:
//!
//! - Владеет [`StateKeeper`] для рекурсивного save/restore
//! - Не generic — один тип для всех компонентов
//! - Передаётся в компонент при создании
//!
//! # Жизненный цикл
//!
//! 1. Корневой `ComponentContext2` создаётся в `Application`
//! 2. При навигации ChildStackManager создаёт дочерние контексты через `child(key)`
//! 3. Компонент получает `ComponentContext2` и может читать свой StateKeeper
//! 4. При destroy — контекст уничтожается, StateKeeper сохраняется в родителе
//!
//! # Поток save/restore
//!
//! ```text
//! Application::on_save_state()
//!   └── root_ctx.state_keeper.save_take()   ← всё дерево рекурсивно
//!
//! Application::on_restore_state()
//!   └── root_ctx.state_keeper.restore(snapshot)  ← всё дерево рекурсивно
//! ```
//!
//! Компонент никогда не вызывает save/restore сам — всё делает контекст.

use crate::state_keeper::StateKeeper;

/// Контекст компонента — владелец StateKeeper.
///
/// Не generic. Один тип для всех компонентов.
/// Предоставляет доступ к рекурсивному save/restore через StateKeeper.
#[derive(Debug)]
pub struct ComponentContext2 {
    /// StateKeeper — рекурсивное дерево состояний.
    state_keeper: StateKeeper,
    /// Ключ этого контекста в родительском StateKeeper.
    key: String,
}

impl ComponentContext2 {
    /// Создать новый корневой контекст.
    ///
    /// Используется в `Application` для корневого компонента.
    pub fn root(key: &str) -> Self {
        Self {
            state_keeper: StateKeeper::new(),
            key: key.to_owned(),
        }
    }

    /// Создать дочерний контекст.
    ///
    /// Создаёт дочерний StateKeeper по ключу в родительском дереве.
    /// Используется ChildStackManager для создания контекстов дочерних компонентов.
    pub fn child(&mut self, _key: &str) -> &mut Self {
        // В реальности нужно создать отдельный ComponentContext2,
        // а не возвращать ссылку на self.
        // Пока заглушка — в 5c будет реализовано правильно.
        todo!("ComponentContext2::child будет реализован в задаче 5c")
    }

    /// Получить ссылку на StateKeeper.
    pub fn state_keeper(&self) -> &StateKeeper {
        &self.state_keeper
    }

    /// Получить мутабельную ссылку на StateKeeper.
    pub fn state_keeper_mut(&mut self) -> &mut StateKeeper {
        &mut self.state_keeper
    }

    /// Получить ключ контекста.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Сохранить состояние (делегирует StateKeeper).
    ///
    /// Забирает владение состоянием из всего дерева.
    /// Вызывается при Destroy Activity.
    pub fn save_all(&mut self) -> crate::state_keeper::StateKeeperSnapshot {
        self.state_keeper.save_take()
    }

    /// Восстановить состояние (делегирует StateKeeper).
    pub fn restore_all(&mut self, snapshot: crate::state_keeper::StateKeeperSnapshot) {
        self.state_keeper.restore(snapshot);
    }

    /// Установить собственное состояние компонента.
    pub fn set_state(&mut self, state: Box<dyn std::any::Any + Send>) {
        self.state_keeper.set_own_state(state);
    }

    /// Получить собственное состояние компонента.
    pub fn state<T: 'static>(&self) -> Option<&T> {
        self.state_keeper.own_state::<T>()
    }

    /// Получить мутабельное собственное состояние.
    pub fn state_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.state_keeper.own_state_mut::<T>()
    }
}

/// Вспомогательный трейт для компонентов, которые используют новый контекст.
///
/// Компонент, созданный через `ComponentFactory2`, получает `&mut ComponentContext2`
/// и должен хранить его для доступа к StateKeeper.
///
/// Реализация по умолчанию — паника (для обратной совместимости).
pub trait ComponentNode2: crate::ComponentNode {
    /// Получить мутабельную ссылку на ComponentContext2.
    ///
    /// Компонент, использующий новый контекст, должен хранить его
    /// и возвращать из этого метода.
    fn context2(&mut self) -> &mut ComponentContext2;
}

// ─── Тесты ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_creation() {
        let ctx = ComponentContext2::root("root");
        assert_eq!(ctx.key(), "root");
        assert!(ctx.state::<()>().is_none());
    }

    #[test]
    fn test_set_and_get_state() {
        let mut ctx = ComponentContext2::root("test");
        ctx.set_state(Box::new(42u32));
        assert_eq!(ctx.state::<u32>(), Some(&42));
    }

    #[test]
    fn test_save_restore_roundtrip() {
        let mut ctx = ComponentContext2::root("root");
        ctx.set_state(Box::new(String::from("hello")));

        let snapshot = ctx.save_all();
        // После save_all состояние забрано
        assert!(ctx.state::<String>().is_none());

        let mut restored = ComponentContext2::root("root");
        restored.restore_all(snapshot);
        assert_eq!(restored.state::<String>(), Some(&"hello".to_string()));
    }

    #[test]
    fn test_state_mut() {
        let mut ctx = ComponentContext2::root("test");
        ctx.set_state(Box::new(vec![1, 2, 3]));

        if let Some(val) = ctx.state_mut::<Vec<i32>>() {
            val.push(4);
        }
        assert_eq!(ctx.state::<Vec<i32>>(), Some(&vec![1, 2, 3, 4]));
    }
}
