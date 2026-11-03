---
name: systems-lowlevel-perf
description: >-
  Low-level performance engineering and hardware-conscious optimization.
  Mastery of CPU cache lines (64B), false sharing prevention, Data-Oriented Design (SoA vs AoS),
  SIMD intrinsics (AVX2/AVX-512/NEON), sanitizers (ASan, TSan, UBSan, MSan), Linux perf,
  flamegraphs, and low-level debugging with GDB/LLDB.
---

# Systems Low-Level Performance & Hardware Optimization

Руководство по низкоуровневой оптимизации, учету архитектуры процессора, устранению гонок и утечек памяти через санитайзеры и профилированию.

---

## ⚡ Ментальная модель «Железо $\leftrightarrow$ Код»

1. **Кэш-линия — квант работы с памятью**: Процессор никогда не читает 1 байт или 1 int. Он всегда загружает из RAM **кэш-линию размером 64 байта**.
2. **Стоимость задержек (Latency Numbers Every Programmer Should Know)**:
   - Регистр: $\approx 0.5$ нс
   - L1 Cache: $\approx 1$ нс (3-4 такта)
   - L2 Cache: $\approx 3-4$ нс (12-14 тактов)
   - L3 Cache: $\approx 15-20$ нс (40-60 тактов)
   - Неудачное предсказание ветвления (Branch mispredict): $\approx 5-15$ нс
   - Оперативная память (RAM main memory): $\approx 60-100$ нс (в 100-200 раз медленнее L1!)
3. **Data-Oriented Design (DOD)**: Данные определяют память, память определяет кэш, кэш определяет реальную производительность.

---

## 🔬 Архитектура данных: AoS vs SoA

Связанные списки (`std::list`), деревья и разрозненные объекты на куче вызывают постоянные Cache Misses («прыжки» по памяти).

### Плохо: Array of Structures (AoS)
Если вам нужно обновить только координаты `x, y`, процессор вынужден тянуть в кэш-линию 200 байт нерелевантных полей каждого юнита:
```cpp
struct Unit {
    float x, y, z;          // 12 байт (нужны в физическом цикле)
    char name[64];          // Не нужны
    int health, mana;       // Не нужны
    std::string equipment;  // Не нужны
};
std::vector<Unit> units;     // Прыгает по памяти, выбивая L1 кэш
```

### Отлично: Structure of Arrays (SoA)
Кэш-линия 64 байта вмещает ровно 16 float-значений подряд, обеспечивая 100% Cache Hit и автоматическую автовекторизацию (SIMD):
```cpp
struct alignas(64) UnitPhysicsSystem {
    std::vector<float> posX;
    std::vector<float> posY;
    std::vector<float> velX;
    std::vector<float> velY;
};
```

---

## 🛑 Борьба с False Sharing (Ложное разделение)

Когда два потока на разных ядрах изменяют независимые переменные, расположенные в **одной кэш-линии (64 байта)**, ядра постоянно сбрасывают кэш друг друга по протоколу когерентности (MESI/MOESI), обрушивая производительность в десятки раз.

**Решение**: выравнивание на размер интерференции:
```cpp
#include <new>

struct ThreadCounter {
    // Гарантирует, что каждый счетчик займет отдельную кэш-линию
    alignas(std::hardware_destructive_interference_size) std::atomic<uint64_t> count{0};
};
```

---

## 🛡️ Квартет Санитайзеров компилятора (Sanitizers)

Никогда не выпускайте нативный код в релиз без прогона под санитайзерами:

| Санитайзер | Флаг компиляции | Что ловит | Накладные расходы |
|---|---|---|---|
| **AddressSanitizer (ASan)** | `-fsanitize=address -fno-omit-frame-pointer` | Use-after-free, переполнение буфера на куче/стеке, double free | $\approx 2\times$ CPU, $2-3\times$ RAM |
| **ThreadSanitizer (TSan)** | `-fsanitize=thread` | Data Races (состояния гонки), Deadlocks между мьютексами | $\approx 3-5\times$ CPU, $5-10\times$ RAM |
| **UndefinedBehavior (UBSan)**| `-fsanitize=undefined` | Знаковое переполнение, деление на 0, битые касты, разыменование null | Минимальные ($\approx 5-10\%$) |
| **MemorySanitizer (MSan)** | `-fsanitize=memory` | Чтение неинициализированной памяти (только под Clang) | $\approx 3\times$ CPU |

> [!WARNING]
> ASan и TSan **несовместимы в одном бинарнике**! Собирайте тесты отдельно под ASan/UBSan и отдельно под TSan.

---

## 📈 Профилирование и Flamegraphs (Linux `perf`)

Не гадайте, где узкое место — профилируйте нативно:

1. **Сбор стеков вызовов с частотой 99 Hz**:
   ```bash
   perf record -F 99 -g -- ./your_highperf_binary
   ```
2. **Генерация Flamegraph (Пламенного графика)**:
   ```bash
   perf script | stackcollapse-perf.pl | flamegraph.pl > flamegraph.svg
   ```
3. **Анализ кэш-промахов и ветвлений**:
   ```bash
   perf stat -e cache-misses,cache-references,branches,branch-misses ./your_binary
   ```
   *Критерий качества*: `branch-misses` должно быть $< 2\%$, `cache-misses` $< 5\%$.
