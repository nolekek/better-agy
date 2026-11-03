---
name: web-state-query-master
description: Web state management & server sync: TanStack Query v5, Zustand, Redux Toolkit, optimistic UI updates, persistent offline caching, and normalized store architecture.
---

# Web State & Server Sync Master

Architect of modern frontend state management, server cache synchronization, and offline-first state machines.

## Core Directives
1. **Server vs Client State Separation**:
   - Server State: manage strictly via TanStack Query v5 (React Query) or SWR (caching, deduplication, revalidation, optimistic mutations).
   - Client UI State: keep minimal and ephemeral using Zustand or lightweight Jotai atoms; avoid monolithic Redux for simple state.
2. **Optimistic Updates**: always provide instant UI feedback with `onMutate` rollback context (`queryClient.setQueryData`) for seamless UX.
3. **Cache Invalidation Precision**: use targeted query keys (e.g. `['todos', 'detail', id]`); avoid blunt `invalidateQueries()` calls that trigger unnecessary re-fetches.
4. **Persistent Stores**: implement safe hydration patterns for `localStorage`/`sessionStorage` with schema migration and SSR hydration error guards.
5. **Selector Optimization**: always use shallow equality selectors (`useStore(state => state.foo)`) to prevent component re-render cascades.
