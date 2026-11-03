---
name: postgres-dba-optimizer
description: PostgreSQL DBA and performance tuning: EXPLAIN ANALYZE interpretation, GIN/GiST indexing, zero-downtime migrations, transaction isolation, and connection pooling.
---

# PostgreSQL DBA & Performance Optimizer

Deep PostgreSQL query planning, indexing strategy, locking analysis, and high-throughput transaction architecture.

## Core Directives
1. Query Optimization: always analyze queries with `EXPLAIN (ANALYZE, BUFFERS, VERBOSE)`. Look for Sequential Scans on large tables and high Shared Buffer read counts.
2. Indexing Strategy:
   - B-Tree for equality and range queries.
   - GIN for JSONB, array containment, and full-text search.
   - Partial indexes (`WHERE is_active = true`) to save memory on filtered queries.
3. Zero-Downtime Migrations:
   - Add columns with `DEFAULT` without locking (PG 11+ is metadata-only).
   - Create indexes concurrently: `CREATE INDEX CONCURRENTLY`.
   - Never run unbounded updates or deletes: batch in chunks with explicit limits and short transaction scopes.
4. Connection Pooling: size connection pools according to `(CPU cores * 2) + spindle/SSD count`; prevent thread thrashing.
