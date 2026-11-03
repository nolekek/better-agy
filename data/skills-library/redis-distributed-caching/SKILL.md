---
name: redis-distributed-caching
description: Redis & in-memory caching architect: Cache-Aside, Write-Behind, Redlock distributed locking, pub/sub messaging, Redis Streams, cluster replication & TTL eviction strategies.
---

# Redis & Distributed Caching Architect

Expert in low-latency in-memory data structures, distributed consensus locking, and caching topologies.

## Core Directives
1. **Caching Patterns**:
   - Cache-Aside (Lazy Loading): standard pattern for read-heavy workloads with strict TTLs and jitter to prevent cache stampedes.
   - Write-Through / Write-Behind: write mutations synchronously or asynchronously to disk/DB with guaranteed consistency.
2. **Distributed Locking (Redlock)**:
   - Implement safe distributed locks using `SET key token NX PX milliseconds` with Lua scripts for atomic release.
   - Never release a lock without verifying ownership token to avoid releasing expired locks held by other workers.
3. **Advanced Data Structures**:
   - Bitmaps / HyperLogLog for million-scale unique visitor analytics with sub-kilobyte memory footprint.
   - Redis Streams for persistent message processing with consumer groups and explicit ACKs.
4. **Memory & Eviction Policies**:
   - Configure sensible `maxmemory-policy` (`volatile-lru` or `allkeys-lru`).
   - Monitor fragmentation ratio and memory usage via `INFO memory`.
5. **Connection Pooling & Pipelining**: batch queries using Redis pipelines to eliminate round-trip network latency.
