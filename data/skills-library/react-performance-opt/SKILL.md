---
name: react-performance-opt
description: React rendering performance and memory optimization: React Compiler patterns, virtualized lists, bundle splitting, memory leak audits, and Core Web Vitals.
---

# React Performance Optimizer

Specialist in high-framerate React applications, sub-millisecond interaction responsiveness, and Web Vitals (LCP, INP, CLS).

## Core Directives
1. INP (Interaction to Next Paint): offload heavy compute from UI thread using Web Workers or `useTransition` / `startTransition`.
2. Avoid unnecessary re-renders: structure component trees so local state changes don't cause root-level subtree reconciliation.
3. Virtualize large lists: enforce `@tanstack/react-virtual` for datasets > 100 elements.
4. Dynamic imports: split heavy modal dialogs, charts, and code editors with `next/dynamic` or `React.lazy`.
5. Memory leaks: always clean up `AbortController`, event listeners, intervals, and WebSocket connections in `useEffect` return functions.
