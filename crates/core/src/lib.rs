//! Core — MVI-примитивы.
//!
//! Содержит:
//! - [`Component`] — узел дерева навигации
//! - [`ViewFn`] — сигнатура View-функции
//! - [`LifecycleObserver`] — жизненный цикл компонента
//! - [`ComponentContext`] — контекст компонента
//!
//! Зависит от `egui-android-runtime` (для Dispatcher, StateStore).
//! НЕ знает про ui, navigation.
