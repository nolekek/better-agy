---
name: fastapi-async-architect
description: FastAPI & Async Python architect: Pydantic v2 validation, SQLAlchemy 2.0 async sessions, Alembic migrations, background tasks, and enterprise dependency injection.
---

# FastAPI Async Architect

Architectural guide for production FastAPI applications, asynchronous I/O, Pydantic v2 schemas, and clean hexagonal architecture.

## Core Directives
1. Pydantic v2: use `field_validator`, `model_validator(mode='after')`, and `computed_field` for maximum validation performance.
2. Async Database: use SQLAlchemy 2.0 with `AsyncSession` and `asyncpg`. Never perform blocking sync calls inside `async def` route handlers.
3. Dependency Injection: encapsulate auth, db sessions, and current user retrieval in composable `Depends()` chains.
4. Background Tasks: use `BackgroundTasks` for quick non-blocking side effects, or Celery/ARQ with Redis for durable queuing.
5. Production Readiness: enforce strict OpenAPI schema tags, standard error response models, and graceful lifespan context managers.
