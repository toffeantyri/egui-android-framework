//! ChildStack — стек дочерних компонентов с управлением жизненным циклом
//! и сохранением/восстановлением состояния.
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
//! # Сохранение/восстановление состояния
//!
//! `ChildStack::save()` сохраняет конфигурацию и состояние всех компонентов
//! в виде `SavedStack<C>`. Каждый компонент возвращает
//! своё состояние через `ComponentNode::save_state()`.
//!
//! `ChildStack::restore_from_saved()` пересоздаёт компоненты через фабрику
//! и восстанавливает их состояние — аналог Decompose.
//!
//! Вызовы save/restore привязаны к Android lifecycle в `Application::on_save_state()`
//! и `Application::on_restore_state()`.

use crate::ComponentFactory;
use egui_android_core::ComponentNode;
use egui_android_runtime::SavedStack;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

/// Стек дочерних компонентов.
pub struct ChildStack<C>
where
    C: Clone + PartialEq,
{
    items: Vec<ChildItem<C>>,
}

struct ChildItem<C> {
    config: C,
    component: Box<dyn ComponentNode>,
}

// ─── Основные методы (без serde) ─────────────────────────────────────────

impl<C> ChildStack<C>
where
    C: Clone + PartialEq + Debug,
{
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

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

    pub fn replace(&mut self, config: C, component: Box<dyn ComponentNode>) {
        if self.items.is_empty() {
            self.push(config, component);
            return;
        }
        let mut old = self.items.pop().unwrap();
        old.component.on_pause();
        old.component.on_stop();
        old.component.on_destroy();

        let mut comp = component;
        comp.on_create();
        comp.on_start();
        comp.on_resume();
        self.items.push(ChildItem {
            config,
            component: comp,
        });
    }

    pub fn bring_to_front(&mut self, config: C, component: Box<dyn ComponentNode>) {
        if let Some(pos) = self.items.iter().position(|item| item.config == config) {
            while self.items.len() > pos + 1 {
                let mut comp = self.items.pop().unwrap().component;
                comp.on_pause();
                comp.on_stop();
                comp.on_destroy();
            }
        } else {
            self.replace(config, component);
        }
    }

    pub fn active(&self) -> Option<&dyn ComponentNode> {
        self.items.last().map(|item| &*item.component)
    }

    pub fn active_mut(&mut self) -> Option<&mut dyn ComponentNode> {
        let idx = self.items.len().checked_sub(1)?;
        Some(&mut *self.items[idx].component)
    }

    pub fn active_config(&self) -> Option<&C> {
        self.items.last().map(|item| &item.config)
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn clear(&mut self) {
        while let Some(removed) = self.items.pop() {
            let mut comp = removed.component;
            comp.on_pause();
            comp.on_stop();
            comp.on_destroy();
        }
    }

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

// ─── Методы сохранения/восстановления (требуют Serialize + Deserialize) ───

impl<C> ChildStack<C>
where
    C: Clone + PartialEq + Debug + Serialize + DeserializeOwned,
{
    /// Сохранить состояние стека. Возвращает сериализуемый `SavedStack<C>`.
    pub fn save(&self) -> SavedStack<C> {
        let items = self
            .items
            .iter()
            .map(|item| {
                let state_bytes = item
                    .component
                    .save_state()
                    .and_then(|boxed| boxed.downcast::<Vec<u8>>().ok().map(|v| *v));
                (item.config.clone(), state_bytes)
            })
            .collect();
        SavedStack { items }
    }

    /// Восстановить стек из сохранённого состояния (пересоздаёт компоненты).
    pub fn restore_from_saved(&mut self, saved: SavedStack<C>, factory: &dyn ComponentFactory<C>) {
        self.clear();
        for (config, state_bytes) in saved.items {
            let mut component = factory.create(config.clone());
            if let Some(bytes) = state_bytes {
                component.restore_state(Box::new(bytes));
            }
            self.push(config, component);
        }
    }

    /// Восстановить состояние без пересоздания (старый метод).
    pub fn restore(&mut self, saved: Vec<(C, Option<Box<dyn std::any::Any + Send>>)>) {
        for (i, (saved_config, state)) in saved.into_iter().enumerate() {
            if let Some(state) = state {
                if let Some(item) = self.items.get_mut(i) {
                    if item.config == saved_config {
                        item.component.restore_state(state);
                    }
                }
            }
        }
    }
}

// ─── Default ────────────────────────────────────────────────────────────────

impl<C> Default for ChildStack<C>
where
    C: Clone + PartialEq + Debug,
{
    fn default() -> Self {
        Self::new()
    }
}

// ─── LifecycleEvent ─────────────────────────────────────────────────────────

pub enum LifecycleEvent {
    Resume,
    Pause,
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use egui_android_core::{Component, LifecycleObserver, UiWrapper};
    use egui_android_runtime::Dispatcher;

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
        fn render(&self, _ui: &mut UiWrapper, _d: &Dispatcher<()>) {}
        fn handle(&mut self, _msg: ()) {}
        fn state(&self) -> &Self::State {
            &()
        }
    }

    #[test]
    fn test_push_lifecycle() {
        let mut s: ChildStack<&str> = ChildStack::new();
        s.push("a", Box::new(TestComp::default()));
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn test_pop_lifecycle() {
        let mut s: ChildStack<&str> = ChildStack::new();
        s.push("a", Box::new(TestComp::default()));
        s.push("b", Box::new(TestComp::default()));
        assert_eq!(s.len(), 2);
        let (cfg, _) = s.pop().unwrap();
        assert_eq!(cfg, "b");
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn test_replace_lifecycle() {
        let mut s: ChildStack<&str> = ChildStack::new();
        s.push("a", Box::new(TestComp::default()));
        s.replace("b", Box::new(TestComp::default()));
        assert_eq!(s.active_config(), Some(&"b"));
    }

    #[test]
    fn test_clear_destroys_all() {
        let mut s: ChildStack<&str> = ChildStack::new();
        s.push("a", Box::new(TestComp::default()));
        s.clear();
        assert!(s.is_empty());
    }

    #[test]
    fn test_pop_empty() {
        assert!(ChildStack::<&str>::new().pop().is_none());
    }

    #[test]
    fn test_active_empty() {
        assert!(ChildStack::<&str>::new().active().is_none());
    }

    #[test]
    fn test_active_mut_empty() {
        assert!(ChildStack::<&str>::new().active_mut().is_none());
    }
}

#[cfg(test)]
#[path = "child_stack_save_tests.rs"]
mod save_tests;
