---
name: lockfree-concurrency-patterns
description: >-
  Lock-free and wait-free concurrency patterns in C++20 and native systems.
  Mastery of C++ memory order (acquire-release, relaxed, seq_cst), CAS loops, ABA mitigation,
  Hazard Pointers, Epoch-Based Reclamation (EBR), and lock-free SPSC ring buffers.
---

# Lock-Free Concurrency & Memory Ordering Architecture

Профессиональное руководство по разработке неблокирующих (lock-free) структур данных, модели памяти C++ и устранению состояний гонки без мьютексов.

---

## ⚡ Модель памяти: Атомики vs Мьютексы

1. **Мьютекс — это системный вызов**: При конкуренции (contention) захват мьютекса переводит поток в спящий режим через вызов ядра ОС (futex/WaitForSingleObject), что стоит от 1000 до 5000 нс (смена контекста).
2. **Lock-Free гарантия**: Как минимум один поток в системе всегда гарантированно прогрессирует (система не может зависнуть целиком, даже если один поток уснул).
3. **`volatile` $\ne$ Потокобезопасность**: Ключевое слово `volatile` указывает компилятору не оптимизировать чтение из памяти (для MMIO регистров), но **НЕ дает атомарности** и **НЕ ставит барьеров памяти** для процессора.

---

## 🧭 Матрица `std::memory_order`

Процессор (особенно ARM и Apple Silicon) и компилятор агрессивно переупорядочивают инструкции чтения и записи для максимальной утилизации конвейера:

| Режим памяти | Что гарантирует | Где применять |
|---|---|---|
| `memory_order_relaxed` | Только атомарность операции (нет барьеров памяти, чтение/запись могут свободно переупорядочиваться) | Метрики, счетчики посещений, генераторы ID |
| `memory_order_release` | Барьер записи: ни одна операция записи **ДО** него не может быть перенесена **ПОСЛЕ** него | Публикация готовых данных / флага готовности |
| `memory_order_acquire` | Барьер чтения: ни одна операция чтения **ПОСЛЕ** него не может быть перенесена **ДО** него | Чтение флага готовности перед чтением данных |
| `memory_order_acq_rel` | Сочетает Acquire и Release для Read-Modify-Write (RMW) операций | `fetch_add`, `compare_exchange` в цикле |
| `memory_order_seq_cst` | Полный глобальный последовательный порядок для всех потоков (ставит тяжелые барьеры MFENCE) | По умолчанию; избегать в hot path высоконагруженных очередей |

---

## 🔄 Идиома Acquire-Release Synchronization

Классический паттерн передачи владения данными между Producer и Consumer без мьютекса:

```cpp
#include <atomic>
#include <string>

struct Payload {
    int data[1024];
};

Payload g_payload;
std::atomic<bool> g_ready{false};

// Поток 1: Producer
void produce() {
    // 1. Заполняем данные
    for (int i = 0; i < 1024; ++i) g_payload.data[i] = i * 42;

    // 2. Публикуем флаг с release семантикой
    // Гарантирует: запись в g_payload не утечет ПОСЛЕ флага g_ready
    g_ready.store(true, std::memory_order_release);
}

// Поток 2: Consumer
void consume() {
    // 1. Ждем флаг с acquire семантикой
    // Гарантирует: чтение g_payload не начнется ДО того, как увидим g_ready == true
    while (!g_ready.load(std::memory_order_acquire)) {
        #if defined(__x86_64__) || defined(_M_X64)
        _mm_pause(); // Снижает нагрузку на конвейер CPU при спинлоке
        #endif
    }

    // 2. Безопасно читаем payload без дата-рейсов
    int val = g_payload.data[0];
}
```

---

## ⚙️ CAS: `compare_exchange_weak` vs `compare_exchange_strong`

- **`compare_exchange_weak` (Предпочтительно в циклах)**: На архитектурах с LL/SC (ARM, RISC-V) операция может завершиться ложным отказом (spurious failure). В цикле `while` слабый CAS компилируется в одну инструкцию `stlex` вместо дорогого цикла валидации сильного CAS.
- **`compare_exchange_strong`**: Используется только если вызов CAS не обернут в цикл и повтор невозможен.

```cpp
// Эталонный цикл CAS (Lock-Free Stack Push):
template <typename T>
void LockFreeStack<T>::push(T val) {
    Node* new_node = new Node(std::move(val));
    new_node->next = head_.load(std::memory_order_relaxed);

    // weak CAS в цикле: если произошел ложный сброс конвейера, мы просто пробуем снова
    while (!head_.compare_exchange_weak(
        new_node->next,
        new_node,
        std::memory_order_release,
        std::memory_order_relaxed)) 
    {
        // new_node->next автоматически обновляется текущим значением head_
    }
}
```

---

## ⚠️ Проблема ABA и Безопасное освобождение памяти

В lock-free стеках и списках, если поток A прочитал `A`, был вытеснен ОС, другие потоки удалили `A`, создали `B`, затем снова вернули память адреса `A` в пул, поток A наивно выполнит CAS, испортив указатели.

### Решения:
1. **Tagged Pointers / Double-Word CAS**: Использование 16-байтного CAS (`cmpxchg16b`) с 64-битным указателем + 64-битным счетчиком поколений (версий).
2. **Hazard Pointers**: Поток перед чтением регистрирует указатель в глобальном массиве «опасных указателей». Деаллокатор проверяет, нет ли указателя в hazard-списке.
3. **Epoch-Based Reclamation (EBR)**: Разделение времени жизни на глобальные эпохи (как в Linux RCU и Rust `crossbeam-epoch`). Память физически освобождается только тогда, когда все активные потоки вышли из старой эпохи.
