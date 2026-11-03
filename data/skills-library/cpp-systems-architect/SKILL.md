---
name: cpp-systems-architect
description: >-
  Modern C++ (C++20/C++23) systems architecture authority. Enforces C++ Core Guidelines,
  strict RAII, Rule of Zero/Five, Concepts, Ranges, Coroutines, monadic error handling (std::expected),
  PMR/custom allocators, zero-overhead abstractions, and modern target-based CMake with Ninja/Presets.
---

# Modern C++ Systems Architect (C++20 / C++23)

Экспертный справочник и архитектурные директивы по разработке надежных и высокопроизводительных нативных систем на современном C++ (C++20 / C++23).

---

## ⚡ Фундаментальные принципы C++

1. **Zero-Overhead Principle**: Вы не платите за то, что не используете. То, что вы используете, невозможно написать вручную эффективнее, чем абстракция компилятора.
2. **Strict RAII (Resource Acquisition Is Initialization)**: Любой ресурс (динамическая память, сокет, файловый дескриптор, мьютекс, GPU buffer) обязан инкапсулироваться в объект с автоматическим деструктором.
3. **Rule of Zero / Rule of Five**:
   - *Rule of Zero (предпочтительно)*: Если класс использует стандартные типы владения (`std::unique_ptr`, `std::string`, `std::vector`), не объявляйте деструктор, конструкторы и операторы копирования/перемещения. Компилятор сгенерирует их корректно.
   - *Rule of Five (для низкоуровневых оберток)*: Если объявлен пользовательский деструктор, обязательно явно определите или удалите все 5 специальных методов:
     ```cpp
     ~Widget();
     Widget(const Widget&) = delete;            // Запрет копирования
     Widget& operator=(const Widget&) = delete;
     Widget(Widget&&) noexcept;                 // Перемещение noexcept обязателен!
     Widget& operator=(Widget&&) noexcept;
     ```
4. **Семантика владения (Ownership Semantics)**:
   - Единственное владение: `std::unique_ptr<T>`.
   - Разделяемое владение (крайняя мера): `std::shared_ptr<T>` + `std::weak_ptr<T>`.
   - Не владеющий доступ: `T&`, `const T&`, сырой указатель `T*` (строго non-owning), `std::string_view`, `std::span<T>`.

---

## 🛠️ Архитектурные паттерны C++20 / C++23

### 1. Concepts & Constraints (вместо SFINAE)
Заменяйте нечитаемый `std::enable_if_t` на чистые концепты C++20:

```cpp
template <typename T>
concept NumericBuffer = requires(T a) {
    { a.data() } -> std::contiguous_iterator;
    { a.size() } -> std::convertible_to<std::size_t>;
    requires std::is_arithmetic_v<typename T::value_type>;
};

template <NumericBuffer T>
void process_telemetry(const T& buffer) {
    // Безопасная и быстрая обработка contiguous данных
}
```

### 2. Monadic Error Handling: `std::expected` (C++23)
В критических по производительности путях (hot path) исключения создают накладные расходы на раскрутку стека (`unwind`). Используйте `std::expected`:

```cpp
#include <expected>
#include <system_error>

enum class NetworkError { Timeout, ConnectionReset, InvalidPayload };

std::expected<Packet, NetworkError> receive_packet(SocketHandle socket) noexcept {
    if (!socket.is_valid()) {
        return std::unexpected(NetworkError::ConnectionReset);
    }
    // ...
    return Packet{/*...*/};
}

// Использование через monadic operations:
auto result = receive_packet(sock)
    .and_then([](const Packet& p) { return parse_payload(p); })
    .or_else([](NetworkError err) { log_error(err); return fallback_packet(); });
```

### 3. Управление памятью: PMR (Polymorphic Memory Resources)
Вместо глобального `malloc`/`new` в циклах используйте аллокаторы без фрагментации:

```cpp
#include <memory_resource>
#include <vector>

void process_frame() {
    std::array<std::byte, 65536> stack_buffer;
    std::pmr::monotonic_buffer_resource mem_pool(stack_buffer.data(), stack_buffer.size());

    // Вектор выделяет память со стека без системных вызовов
    std::pmr::vector<OrderBookEntry> orders(&mem_pool);
    orders.reserve(1024);
    // Деструктор mem_pool мгновенно освободит всю память целиком
}
```

---

## 🏗️ Стандарты CMake: Modern Target-Based Architecture

Никаких устаревших `include_directories()` и глобальных переменных. Только строгая target-based изоляция:

```cmake
cmake_minimum_required(VERSION 3.25)
project(HighPerfEngine LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Создание строгого интерфейсного/статического таргета
add_library(engine_core STATIC
    src/core.cpp
    src/buffer_pool.cpp
)

target_include_directories(engine_core
    PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
        $<INSTALL_INTERFACE:include>
    PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}/src
)

target_compile_features(engine_core PUBLIC cxx_std_20)

# Строгие флаги предупреждений компилятора
if (MSVC)
    target_compile_options(engine_core PRIVATE /W4 /WX /permissive-)
else()
    target_compile_options(engine_core PRIVATE -Wall -Wextra -Wpedantic -Werror -Wshadow)
endif()

# Подключение санитайзеров для отладочной сборки
option(ENABLE_ASAN "Enable AddressSanitizer" OFF)
if (ENABLE_ASAN AND NOT MSVC)
    target_compile_options(engine_core PUBLIC -fsanitize=address,undefined -fno-omit-frame-pointer)
    target_link_options(engine_core PUBLIC -fsanitize=address,undefined)
endif()
```

---

## 🧪 Чек-лист проверки C++ кода перед коммитом

- [ ] Конструкторы, принимающие 1 аргумент, помечены как `explicit` (защита от неявных преобразований).
- [ ] Перемещающие конструкторы и операторы присваивания помечены `noexcept` (иначе `std::vector` будет копировать объекты при realloc).
- [ ] Все полиморфные базовые классы имеют `virtual ~Base() = default;`.
- [ ] В `switch` по `enum` обработаны все варианты или включен `-Wswitch` без хаотичного `default:`.
- [ ] Сырые указатели используются ТОЛЬКО как невладеющие ссылки, никогда не удаляются через `delete`.
- [ ] В hot path нет скрытых динамических аллокаций (`std::string`, `std::function`, нерезервированные `std::vector`).
