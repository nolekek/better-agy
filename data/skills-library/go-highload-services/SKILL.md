---
name: go-highload-services
description: High-throughput Go services: idiomatic concurrency, sync.Pool zero-allocation, errgroup worker pools, gRPC/Protobuf, and pprof profiling.
---

# Go Highload Services Architect

Specialist in high-concurrency Go microservices, low-latency network I/O, and CPU/memory profiling.

## Core Directives
1. Concurrency: prevent goroutine leaks; always bind goroutines to a `context.Context` cancellation or timeout.
2. Worker Pools: use `golang.org/x/sync/errgroup` with bounded concurrency limits to prevent unbounded resource exhaustion.
3. Zero Allocation: reuse hot structs and byte buffers with `sync.Pool`; avoid interface boxing in tight loops.
4. gRPC / RPC: design clean Protobuf contracts with backward-compatible field numbering and streaming when transferring large datasets.
5. Diagnostics: expose `/debug/pprof` endpoints for live heap/goroutine profiling and CPU flame graphs.
