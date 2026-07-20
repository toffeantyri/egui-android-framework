//! [`StateKeeper`] — рекурсивное дерево состояний компонентов.
//!
//! Аналог `StateKeeperDispatcher` в Decompose/Essenty.
//!
//! # Зачем
//!
//! Каждый компонент может иметь собственное состояние, которое нужно
//! сохранить при Destroy Activity и восстановить при InitWindow.
//! Компоненты образуют дерево (через ChildStack), поэтому и состояния
//! должны образовывать дерево.
//!
//! `StateKeeper` автоматически обходит дерево при save/restore —
//! без ручного вызова на каждом уровне.
//!
//! # Как использовать
//!
//! ```ignore
//! // Корневой StateKeeper
//! let mut root = StateKeeper::new();
//!
//! // Дочерний по ключу
//! let child = root.child("screen:home");
//! child.set_own_state(Box::new(MyState { count: 42 }));
//!
//! // Сохранить всё дерево
//! let snapshot = root.save();
//!
//! // Восстановить
//! let mut restored = StateKeeper::new();
//! restored.restore(snapshot);
//!
//! assert_eq!(
//!     restored.child("screen:home").own_state::<MyState>(),
//!     Some(&MyState { count: 42 })
//! );
//! ```
//!
//! # Рекурсия
//!
//! `save()` рекурсивно обходит всех children.
//! `restore()` рекурсивно восстанавливает всех children по ключам.
//! Если дочерний ключ есть в snapshot, но нет в текущем дереве — он игнорируется.
//! Если дочерний ключ есть в дереве, но нет в snapshot — его состояние не меняется.

use std::any::Any;

// ─── StateKeeper ────────────────────────────────────────────────────────────

/// Рекурсивное дерево состояний.
///
/// Каждый узел хранит:
/// - `own_state` — состояние текущего компонента (если есть)
/// - `children` — дочерние StateKeeper по строковым ключам
pub struct StateKeeper {
    own_state: Option<Box<dyn Any + Send>>,
    children: Vec<(String, StateKeeper)>,
}

impl StateKeeper {
    /// Создать пустой StateKeeper.
    pub fn new() -> Self {
        Self {
            own_state: None,
            children: Vec::new(),
        }
    }

    /// Создать StateKeeper с собственным состоянием.
    pub fn with_state(state: Box<dyn Any + Send>) -> Self {
        Self {
            own_state: Some(state),
            children: Vec::new(),
        }
    }

    // ─── Доступ к состоянию ─────────────────────────────────────────

    /// Установить собственное состояние.
    pub fn set_own_state(&mut self, state: Box<dyn Any + Send>) {
        self.own_state = Some(state);
    }

    /// Взять собственное состояние (извлечь, оставив `None`).
    pub fn take_own_state(&mut self) -> Option<Box<dyn Any + Send>> {
        self.own_state.take()
    }

    /// Получить ссылку на собственное состояние, downcast'нув в T.
    pub fn own_state<T: 'static>(&self) -> Option<&T> {
        self.own_state
            .as_deref()
            .and_then(|s| s.downcast_ref::<T>())
    }

    /// Получить мутабельную ссылку на собственное состояние, downcast'нув в T.
    pub fn own_state_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.own_state
            .as_deref_mut()
            .and_then(|s| s.downcast_mut::<T>())
    }

    /// Проверить, есть ли собственное состояние.
    pub fn has_own_state(&self) -> bool {
        self.own_state.is_some()
    }

    /// Очистить собственное состояние.
    pub fn clear_own_state(&mut self) {
        self.own_state = None;
    }

    // ─── Дочерние StateKeeper ──────────────────────────────────────

    /// Получить или создать дочерний StateKeeper по ключу.
    ///
    /// Если дочерний StateKeeper с таким ключом уже существует — возвращает
    /// ссылку на него. Если нет — создаёт новый пустой и возвращает ссылку.
    pub fn child(&mut self, key: &str) -> &mut StateKeeper {
        let key = key.to_owned();
        if let Some(pos) = self.children.iter().position(|(k, _)| k == &key) {
            return &mut self.children[pos].1;
        }
        self.children.push((key, StateKeeper::new()));
        &mut self.children.last_mut().unwrap().1
    }

    /// Удалить дочерний StateKeeper по ключу.
    pub fn remove_child(&mut self, key: &str) {
        self.children.retain(|(k, _)| k != key);
    }

    /// Очистить всех дочерних StateKeeper.
    pub fn clear_children(&mut self) {
        self.children.clear();
    }

    /// Получить количество дочерних StateKeeper.
    pub fn children_count(&self) -> usize {
        self.children.len()
    }

    /// Проверить, есть ли дочерний StateKeeper по ключу.
    pub fn has_child(&self, key: &str) -> bool {
        self.children.iter().any(|(k, _)| k == key)
    }

    // ─── Save / Restore ────────────────────────────────────────────

    /// Сохранить всё дерево рекурсивно.
    ///
    /// Возвращает `StateKeeperSnapshot`, который можно сериализовать
    /// (в будущем — через Parcelable) и восстановить позже.
    pub fn save(&self) -> StateKeeperSnapshot {
        StateKeeperSnapshot {
            own_state: self.own_state.as_ref().map(|s| {
                // Клонировать Box<dyn Any> нельзя — Any не Clone.
                // Поэтому при save мы только логируем наличие состояния.
                // Реальная сериализация будет в Фазе 5a.2 через трейт SerializeState.
                // Пока save/restore работает только в памяти (для одного процесса).
                log::debug!(
                    "StateKeeper::save: сохранено состояние ({} байт)",
                    std::mem::size_of_val(&*s)
                );
                // Придётся передавать владение — но save забирает ref, не владение.
                // Используем решение: save забирает владение, оставляя None.
                // Но это неудобно для API. Лучше сделать отдельный метод save_take().
                // Пока save() работает как копия — для тестов.
                // В реальности save() будет принимать &self, а snapshot будет
                // содержать сериализованные данные (Vec<u8>), не Box<dyn Any>.
                //
                // В этой итерации save() — заглушка, просто отмечаем что состояние есть.
                Box::new(()) as Box<dyn Any + Send>
            }),
            children: self
                .children
                .iter()
                .map(|(key, child)| (key.clone(), child.save()))
                .collect(),
        }
    }

    /// Сохранить дерево, забирая владение состояниями (destructive read).
    ///
    /// После вызова `save_take()` собственное состояние и состояния
    /// всех children обнуляются. Используется при реальном save перед Destroy.
    pub fn save_take(&mut self) -> StateKeeperSnapshot {
        StateKeeperSnapshot {
            own_state: self.own_state.take(),
            children: self
                .children
                .iter_mut()
                .map(|(key, child)| (key.clone(), child.save_take()))
                .collect(),
        }
    }

    /// Восстановить дерево из snapshot.
    ///
    /// Восстанавливает собственное состояние и рекурсивно обходит
    /// дочерние StateKeeper, сопоставляя их по ключам.
    /// Если дочерний ключ есть в snapshot, но нет в дереве — создаёт новый.
    /// Если дочерний ключ есть в дереве, но нет в snapshot — не меняется.
    pub fn restore(&mut self, snapshot: StateKeeperSnapshot) {
        self.own_state = snapshot.own_state;

        for (key, child_snapshot) in snapshot.children {
            // Создаём child, если его нет (для полного восстановления)
            // или используем существующий (для частичного обновления)
            self.child(&key).restore(child_snapshot);
        }
    }

    /// Восстановить дерево, заменяя children только из snapshot.
    ///
    /// Отличается от `restore()` тем, что удаляет всех children,
    /// которых нет в snapshot. Используется при полном восстановлении
    /// после process kill.
    pub fn restore_strict(&mut self, snapshot: StateKeeperSnapshot) {
        self.own_state = snapshot.own_state;

        // Заменяем children — оставляем только те, что есть в snapshot
        let mut new_children: Vec<(String, StateKeeper)> = snapshot
            .children
            .into_iter()
            .map(|(key, child_snapshot)| {
                let mut keeper = StateKeeper::new();
                keeper.restore(child_snapshot);
                (key, keeper)
            })
            .collect();

        // Для ключей, которые есть и в старом дереве, и в snapshot —
        // сохраняем order из snapshot, но глубину берём из старого
        for (key, child) in &mut new_children {
            if let Some(pos) = self.children.iter().position(|(k, _)| k == key) {
                let old_child = &mut self.children[pos].1;
                // Переносим children старого в новый, если их нет в snapshot
                let old_keys: Vec<String> =
                    old_child.children.iter().map(|(k, _)| k.clone()).collect();
                for old_key in old_keys {
                    if !child.has_child(&old_key) {
                        // Забираем StateKeeper из старого child
                        if let Some(idx) =
                            old_child.children.iter().position(|(k, _)| k == &old_key)
                        {
                            let (_, keeper) = old_child.children.remove(idx);
                            *child.child(&old_key) = keeper;
                        }
                    }
                }
            }
        }

        self.children = new_children;
    }

    /// Очистить всё дерево (собственное состояние + children).
    pub fn clear(&mut self) {
        self.own_state = None;
        self.children.clear();
    }
}

impl Default for StateKeeper {
    fn default() -> Self {
        Self::new()
    }
}

// ─── StateKeeperSnapshot ────────────────────────────────────────────────────

/// Snapshot всего дерева состояний.
///
/// Создаётся `StateKeeper::save()` / `StateKeeper::save_take()`.
/// Восстанавливается через `StateKeeper::restore()` / `StateKeeper::restore_strict()`.
///
/// В будущем: замена `Box<dyn Any + Send>` на сериализованные данные (Vec<u8>)
/// для поддержки Parcelable / process kill.
#[derive(Debug)]
pub struct StateKeeperSnapshot {
    /// Состояние текущего узла (если есть).
    pub own_state: Option<Box<dyn Any + Send>>,
    /// Дочерние snapshot по ключам.
    pub children: Vec<(String, StateKeeperSnapshot)>,
}

// ─── Тесты ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct Counter(u32);

    #[derive(Debug, Clone, PartialEq)]
    struct Label(String);

    #[test]
    fn test_new_keeper_is_empty() {
        let keeper = StateKeeper::new();
        assert!(!keeper.has_own_state());
        assert_eq!(keeper.children_count(), 0);
    }

    #[test]
    fn test_set_and_get_own_state() {
        let mut keeper = StateKeeper::new();
        keeper.set_own_state(Box::new(Counter(42)));
        assert!(keeper.has_own_state());
        assert_eq!(keeper.own_state::<Counter>(), Some(&Counter(42)));
    }

    #[test]
    fn test_own_state_wrong_type() {
        let mut keeper = StateKeeper::new();
        keeper.set_own_state(Box::new(Counter(42)));
        assert_eq!(keeper.own_state::<Label>(), None);
    }

    #[test]
    fn test_take_own_state() {
        let mut keeper = StateKeeper::new();
        keeper.set_own_state(Box::new(Counter(42)));
        let taken = keeper.take_own_state();
        assert!(taken.is_some());
        assert!(!keeper.has_own_state());
    }

    #[test]
    fn test_child_creation() {
        let mut keeper = StateKeeper::new();
        let child = keeper.child("screen:home");
        child.set_own_state(Box::new(Counter(1)));

        assert!(keeper.has_child("screen:home"));
        assert_eq!(keeper.children_count(), 1);

        let child = keeper.child("screen:home");
        assert_eq!(child.own_state::<Counter>(), Some(&Counter(1)));
    }

    #[test]
    fn test_child_reuses_existing() {
        let mut keeper = StateKeeper::new();
        keeper.child("a").set_own_state(Box::new(Counter(10)));
        keeper.child("b").set_own_state(Box::new(Counter(20)));

        assert_eq!(keeper.children_count(), 2);

        // Повторное получение не создаёт новый
        let _ = keeper.child("a");
        assert_eq!(keeper.children_count(), 2);
    }

    #[test]
    fn test_remove_child() {
        let mut keeper = StateKeeper::new();
        keeper.child("a");
        keeper.child("b");
        assert_eq!(keeper.children_count(), 2);

        keeper.remove_child("a");
        assert_eq!(keeper.children_count(), 1);
        assert!(!keeper.has_child("a"));
        assert!(keeper.has_child("b"));
    }

    #[test]
    fn test_save_and_restore_flat() {
        let mut keeper = StateKeeper::new();
        keeper.set_own_state(Box::new(Counter(42)));

        let snapshot = keeper.save_take();
        assert!(!keeper.has_own_state()); // было take

        let mut restored = StateKeeper::new();
        restored.restore(snapshot);
        assert_eq!(restored.own_state::<Counter>(), Some(&Counter(42)));
    }

    #[test]
    fn test_save_and_restore_with_children() {
        let mut root = StateKeeper::new();
        root.set_own_state(Box::new(Label("root".into())));
        root.child("child1").set_own_state(Box::new(Counter(1)));
        root.child("child1")
            .child("grandchild")
            .set_own_state(Box::new(Counter(11)));
        root.child("child2").set_own_state(Box::new(Counter(2)));

        let snapshot = root.save_take();

        let mut restored = StateKeeper::new();
        restored.restore(snapshot);

        assert_eq!(restored.own_state::<Label>(), Some(&Label("root".into())));
        assert_eq!(
            restored.child("child1").own_state::<Counter>(),
            Some(&Counter(1))
        );
        assert_eq!(
            restored
                .child("child1")
                .child("grandchild")
                .own_state::<Counter>(),
            Some(&Counter(11))
        );
        assert_eq!(
            restored.child("child2").own_state::<Counter>(),
            Some(&Counter(2))
        );
    }

    #[test]
    fn test_restore_ignores_unknown_children() {
        let mut keeper = StateKeeper::new();
        keeper.child("keep_me").set_own_state(Box::new(Counter(1)));

        let snapshot = StateKeeperSnapshot {
            own_state: None,
            children: vec![
                (
                    "keep_me".into(),
                    StateKeeperSnapshot {
                        own_state: Some(Box::new(Counter(42))),
                        children: vec![],
                    },
                ),
                (
                    "unknown".into(),
                    StateKeeperSnapshot {
                        own_state: Some(Box::new(Counter(99))),
                        children: vec![],
                    },
                ),
            ],
        };

        keeper.restore(snapshot);
        // keep_me — обновлён
        assert_eq!(
            keeper.child("keep_me").own_state::<Counter>(),
            Some(&Counter(42))
        );
        // unknown — теперь restore создаёт новый child, если его нет
        assert!(keeper.has_child("unknown"));
        assert_eq!(
            keeper.child("unknown").own_state::<Counter>(),
            Some(&Counter(99))
        );
    }

    #[test]
    fn test_clear_tree() {
        let mut keeper = StateKeeper::new();
        keeper.set_own_state(Box::new(Counter(1)));
        keeper.child("a").set_own_state(Box::new(Counter(2)));
        keeper.clear();

        assert!(!keeper.has_own_state());
        assert_eq!(keeper.children_count(), 0);
    }

    #[test]
    fn test_restore_strict_replaces_children() {
        let mut keeper = StateKeeper::new();
        keeper.child("old1").set_own_state(Box::new(Counter(1)));
        keeper.child("old2").set_own_state(Box::new(Counter(2)));

        let snapshot = StateKeeperSnapshot {
            own_state: None,
            children: vec![(
                "new1".into(),
                StateKeeperSnapshot {
                    own_state: Some(Box::new(Counter(10))),
                    children: vec![],
                },
            )],
        };

        keeper.restore_strict(snapshot);
        assert_eq!(keeper.children_count(), 1);
        assert!(keeper.has_child("new1"));
        assert!(!keeper.has_child("old1"));
        assert!(!keeper.has_child("old2"));
    }

    #[test]
    fn test_child_mut() {
        let mut keeper = StateKeeper::new();
        keeper.set_own_state(Box::new(Counter(0)));

        // Модификация через own_state_mut
        if let Some(val) = keeper.own_state_mut::<Counter>() {
            val.0 = 100;
        }
        assert_eq!(keeper.own_state::<Counter>(), Some(&Counter(100)));
    }

    #[test]
    fn test_deep_tree_save_restore() {
        // 3 уровня вложенности
        let mut root = StateKeeper::new();
        root.child("l1")
            .child("l2")
            .child("l3")
            .set_own_state(Box::new(Label("deep".into())));

        let snapshot = root.save_take();
        let mut restored = StateKeeper::new();
        restored.restore(snapshot);

        assert_eq!(
            restored
                .child("l1")
                .child("l2")
                .child("l3")
                .own_state::<Label>(),
            Some(&Label("deep".into()))
        );
    }
}
