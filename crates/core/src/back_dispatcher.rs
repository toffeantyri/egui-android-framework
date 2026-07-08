//! BackDispatcher — центральный менеджер обработки системной кнопки Back.
//!
//! Аналог `BackDispatcher` из Decompose.
//!
//! Позволяет компонентам регистрировать обработчики Back с приоритетами.
//! При нажатии Back обработчики вызываются от высокого приоритета к низкому.
//! Первый, кто вернул `true`, перехватывает Back.
//! Если никто не перехватил — `handle()` возвращает `false`, и вызывающий
//! (обычно ChildStack) делает `pop()`.

/// Callback для обработки Back.
///
/// Возвращает `true`, если Back перехвачен (событие обработано).
/// Возвращает `false`, если Back не обработан (нужно передать дальше).
pub struct BackCallback {
    /// Приоритет: чем выше число, тем раньше вызывается.
    /// Рекомендации:
    /// - 100 — диалоги
    /// - 50 — вложенные стеки (nested ChildStack)
    /// - 10 — сам компонент
    /// - 0 — системное поведение (pop)
    pub priority: u32,
    /// Обработчик Back. Получает `()`, возвращает `bool`.
    pub handler: Box<dyn FnMut() -> bool>,
}

impl std::fmt::Debug for BackCallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BackCallback")
            .field("priority", &self.priority)
            .finish()
    }
}

/// Центральный менеджер обработки Back.
///
/// Хранит список callbacks, отсортированных по приоритету.
/// При `handle()` вызывает их от высокого приоритета к низкому.
#[derive(Default)]
pub struct BackDispatcher {
    callbacks: Vec<BackCallback>,
}

impl BackDispatcher {
    /// Создать пустой диспетчер.
    pub fn new() -> Self {
        Self {
            callbacks: Vec::new(),
        }
    }

    /// Зарегистрировать обработчик Back.
    ///
    /// После регистрации callbacks сортируются по приоритету (возрастание).
    /// При `handle()` вызываются в обратном порядке (от высокого к низкому).
    pub fn register(&mut self, callback: BackCallback) {
        self.callbacks.push(callback);
        self.callbacks.sort_by_key(|c| c.priority);
    }

    /// Удалить все обработчики с указанным приоритетом.
    pub fn unregister(&mut self, priority: u32) {
        self.callbacks.retain(|c| c.priority != priority);
    }

    /// Обработать Back.
    ///
    /// Вызывает обработчики от высокого приоритета к низкому.
    /// Если хотя бы один вернул `true` — возвращает `true` (обработано).
    /// Если ни один не обработал — возвращает `false`.
    pub fn handle(&mut self) -> bool {
        // Вызываем в обратном порядке (от высокого приоритета к низкому)
        for cb in self.callbacks.iter_mut().rev() {
            if (cb.handler)() {
                return true;
            }
        }
        false
    }

    /// Количество зарегистрированных обработчиков.
    pub fn len(&self) -> usize {
        self.callbacks.len()
    }

    /// Пуст ли список обработчиков.
    pub fn is_empty(&self) -> bool {
        self.callbacks.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_returns_false_when_no_callbacks() {
        let mut d = BackDispatcher::new();
        assert!(!d.handle());
    }

    #[test]
    fn test_handle_calls_priority_order() {
        let mut d = BackDispatcher::new();
        let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

        {
            let o = order.clone();
            d.register(BackCallback {
                priority: 10,
                handler: Box::new(move || {
                    o.lock().unwrap().push(10);
                    false
                }),
            });
        }
        {
            let o = order.clone();
            d.register(BackCallback {
                priority: 20,
                handler: Box::new(move || {
                    o.lock().unwrap().push(20);
                    false
                }),
            });
        }
        {
            let o = order.clone();
            d.register(BackCallback {
                priority: 5,
                handler: Box::new(move || {
                    o.lock().unwrap().push(5);
                    false
                }),
            });
        }

        d.handle();
        // Ожидаемый порядок: 20 (высший), 10, 5 (низший)
        assert_eq!(*order.lock().unwrap(), vec![20, 10, 5]);
    }

    #[test]
    fn test_handle_stops_at_first_true() {
        let mut d = BackDispatcher::new();
        let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));

        {
            let o = order.clone();
            d.register(BackCallback {
                priority: 10,
                handler: Box::new(move || {
                    o.lock().unwrap().push(10);
                    false
                }),
            });
        }
        {
            let o = order.clone();
            d.register(BackCallback {
                priority: 20,
                handler: Box::new(move || {
                    o.lock().unwrap().push(20);
                    true
                }),
            });
        }
        {
            let o = order.clone();
            d.register(BackCallback {
                priority: 5,
                handler: Box::new(move || {
                    o.lock().unwrap().push(5);
                    false
                }),
            });
        }

        assert!(d.handle());
        assert_eq!(*order.lock().unwrap(), vec![20]);
    }

    #[test]
    fn test_register_unregister() {
        let mut d = BackDispatcher::new();
        d.register(BackCallback {
            priority: 10,
            handler: Box::new(|| true),
        });
        assert_eq!(d.len(), 1);

        d.unregister(10);
        assert!(d.is_empty());
    }
}
