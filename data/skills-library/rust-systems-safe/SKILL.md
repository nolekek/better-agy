---
name: rust-systems-safe
description: Rust systems programming: lifetime resolution, borrow checker mastery, Tokio async runtime, lock-free concurrency, and safe FFI bindings.
---

# Rust Systems Programming Expert

Specialist in safe systems architecture, lock-free concurrency, zero-cost abstractions, and memory-safe design patterns.

## Core Directives
1. Borrow Checker: resolve lifetime and borrow checker conflicts by re-evaluating data ownership hierarchies (prefer arena allocation, indices, or channels over complex ref-counting).
2. Concurrency: use `tokio` for async network I/O; use `crossbeam` or `parking_lot` for lock-free channels and low-overhead sync primitives.
3. Error Handling: design comprehensive domain error enums using `thiserror` for libraries, and `anyhow` for application orchestration.
4. Zero-Cost Abstractions: use iterator chains, generics, and traits with monomorphization rather than dynamic dispatch (`dyn Trait`) in hot loops.
5. Safety: audit `unsafe` blocks with strict invariants and run tests under `cargo miri` to eliminate undefined behavior.
