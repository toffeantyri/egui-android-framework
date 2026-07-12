# UI Test Contract v1.0 — Полная спецификация тестов для egui-android-ui

## Цель

Контракт гарантирует полное покрытие всех UI-элементов, модификаторов, контейнеров и пайплайна Measure → Layout → Paint тестами, включая все краевые случаи.

---

## 1. Measure Phase

Каждый виджет обязан измерить контент и получить точный `content_size`.

### 1.1 Точный размер контента
- **Text:** `rect.height == galley.size().y`
- **Spacer:** размер = заданный
- **Icon:** размер = размер изображения

### 1.2 Учёт модификаторов размера
- `wrap_content_width` / `wrap_content_size` — размер по контенту
- `width` / `height` — фиксированный размер
- `width_in` / `height_in` — clamp по min/max
- `fill_max_width` — min_width = available.x
- `fill_max_size` — min/max по обеим осям

### 1.3 Учёт модификаторов внешнего вида
- `padding`: consum = +2×padding
- `border`: consum = +2×border_width
- `background`: не меняет consum
- `clip`: не меняет consum
- `shadow`: не меняет consum
- `alpha`: не меняет consum

### 1.4 DPI-инвариантность
- pp = 1.0
- pp = 3.25
- consum не должен зависеть от pp

---

## 2. Layout Phase

### 2.1 Корректный alloc
- alloc строго по content_size
- clamp по constraints
- отсутствие двойного alloc (Text НЕ вызывает два alloc подряд)

### 2.2 Размещение в контейнерах
- **Column:** consum = сумма consum детей + spacing
- **Row:** consum по ширине = сумма ширин детей + spacing
- **Stack:** consum = max(children)
- **LazyColumn:** корректное потребление высоты
- **Scrollable Column:** fill_max_width работает внутри скролла

### 2.3 Nested containers
- Column внутри Row
- Row внутри Column
- Stack внутри Column
- LazyColumn внутри ScrollArea

---

## 3. Paint Phase

### 3.1 Рисование строго внутри rect
- painter_at(rect) рисует внутри выделенной области
- background/border/shadow не выходят за пределы rect

### 3.2 Модификаторы визуального уровня
- background, border, clip, shadow, alpha — не меняют layout, только paint

---

## 4. Modifier Pipeline

### 4.1 Порядок модификаторов
- padding + background
- background + padding
- padding + border + background
- clickable + padding + background
- width + height + background
- fill_max_width + background + border + padding

### 4.2 Комбинации модификаторов
- wrap_content_size + border + background
- fill_max_width + padding
- width_in + height_in + background
- clickable + padding + background
- цепочка из 5+ модификаторов

### 4.3 Отрицательные и нулевые значения
- padding(0)
- padding(-10)
- border(0)
- size(0)
- shadow(0)
- ни один не должен паниковать

---

## 5. Containers

### 5.1 Column
- top-down layout
- spacing
- nested modifiers
- scrollable

### 5.2 Row
- left-right layout
- wrap_content_width
- nested modifiers

### 5.3 Stack
- max(children)
- overlay
- modifiers

### 5.4 LazyColumn
- корректное потребление высоты
- clickable внутри LazyColumn

---

## 6. Text

- rect.height == galley.height
- wrap_content_width == ширина текста
- multiline (перенос строк)
- DPI-инвариантность
- background + border + padding + wrap_content_size
- align(Center) внутри fill_max_width

---

## 7. SizedWidget / Button

- width
- height
- width + height
- width_in + height_in
- background + border + width + height
- Button клик → dispatch message
- Button смена цвета pressed/normal

---

## 8. Regression

- отсутствие двойного render
- отсутствие двойного alloc
- отсутствие паники в любых комбинациях модификаторов
- отсутствие паники в любых контейнерах
- корректное наследование constraints через Frame::show

---

## 9. Full Coverage Matrix

| Элемент | Measure | Layout | Paint | Modifiers | DPI | Negative | Nested |
|---------|---------|--------|-------|-----------|-----|----------|--------|
| All widgets | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| All modifiers | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |

---

## 10. Правило для агента

При доработке тестов агент обязан:
1. Определить, какие секции контракта затрагиваются изменениями
2. Убедиться, что для каждого затронутого элемента есть тесты по всей матрице
3. Убедиться, что тесты покрывают pp=1.0 и pp=3.25
4. Убедиться, что тесты покрывают отрицательные/нулевые значения
5. Добавить недостающие тесты, если контракт не выполнен
