//! [`ComponentFactory`] — фабрика компонентов по конфигурации.
//!
//! Выделяет логику создания компонентов из `NavigationHost`,
//! позволяя реализовать Open/Closed Principle (OCP):
//! новый экран = новая реализация `ComponentFactory`, а не правка `NavigationHost`.
//!
//! # Пример
//!
//! ```ignore
//! use egui_android_navigation::ComponentFactory;
//!
//! struct MyFactory;
//!
//! impl ComponentFactory<Route> for MyFactory {
//!     fn create(&self, config: Route) -> Box<dyn ComponentNode> {
//!         match config {
//!             Route::Home => Box::new(HomeScreen::new()),
//!             Route::Settings => Box::new(SettingsScreen::new()),
//!         }
//!     }
//! }
//! ```

use egui_android_core::component_context2::ComponentContext2;
use egui_android_core::ComponentNode;

/// Фабрика компонентов по конфигурации.
///
/// Аналог `Router` в Decompose: по конфигурации (маршруту) создаёт
/// соответствующий компонент. Это позволяет отделить фабричную логику
/// от хоста навигации и соблюсти OCP.
///
/// # Параметры типа
///
/// * `C` — конфигурация (маршрут), по которой фабрика создаёт компонент.
///   Обычно это enum вроде `Route`.
pub trait ComponentFactory<C> {
    /// Создать компонент для данной конфигурации.
    fn create(&self, config: C) -> Box<dyn ComponentNode>;
}

/// Фабрика компонентов с передачей ComponentContext2.
///
/// Отличается от [`ComponentFactory`] тем, что передаёт в компонент
/// [`ComponentContext2`] при создании. Это позволяет компоненту
/// получить доступ к StateKeeper для рекурсивного save/restore.
///
/// Используется с [`super::child_stack_manager::ChildStackManager`].
pub trait ComponentFactory2<C> {
    /// Создать компонент с контекстом.
    fn create(&self, config: C, ctx: ComponentContext2) -> Box<dyn ComponentNode>;
}

/// Создаёт компонент через `ComponentFactory` и оборачивает его с lifecycle-вызовами.
///
/// Удобная обёртка: `factory.create(config)` — без необходимости
/// каждый раз вызывать `on_create`/`on_start`/`on_resume` вручную.
/// Если компонент уже проходит lifecycle через `ChildStack::push()`,
/// используйте напрямую `factory.create()`.
pub fn create_component<C>(factory: &dyn ComponentFactory<C>, config: C) -> Box<dyn ComponentNode> {
    let mut component = factory.create(config);
    component.on_create();
    component.on_start();
    component.on_resume();
    component
}
