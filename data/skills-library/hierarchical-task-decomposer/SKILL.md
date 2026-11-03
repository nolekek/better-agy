---
name: hierarchical-task-decomposer
description: "Decomposes monolithic requests and large-scale architectures into MECE subtasks, directed acyclic graphs (DAGs), strict contract interfaces, and clear file ownership boundaries for multi-agent workflows."
risk: low
source: antigravity-orchestrator-core (Apache 2.0)
date_added: 2026-10-31
---

# Hierarchical Task Decomposer

The **Hierarchical Task Decomposer** skill transforms large, underspecified, or complex user goals into atomic, dependency-ordered work units suitable for concurrent or sequential execution by specialized AI agents.

---

## 🎯 Core Principles

1. **MECE Breakdown (Mutually Exclusive, Collectively Exhaustive)**:
   - Subtasks must not overlap in scope or file ownership.
   - The union of all subtasks must completely fulfill the user's objective without missing requirements.

2. **Strict Contract Boundaries First**:
   - Before any subagent starts writing implementation code, all interfaces, data schemas (JSON schema / TypeScript types), and API contracts must be locked down.
   - Subagents build against agreed contracts, eliminating integration friction.

3. **File & Domain Ownership Partitioning**:
   - Assign exclusive file paths or directories to specific subtasks (e.g., Worker A owns `src/backend/`, Worker B owns `src/frontend/components/`).
   - Shared configuration files (`package.json`, root `index.js`) must be modified only by the orchestrator or in sequentially gated steps.

4. **Directed Acyclic Graph (DAG) Sequencing**:
   - Classify subtasks by dependencies:
     - **Level 0 (Foundation)**: Schema design, contract definitions, environment scaffolding.
     - **Level 1 (Core Implementations)**: Independent domain modules (Backend endpoints, UI views).
     - **Level 2 (Integration & E2E)**: Playwright tests, load testing, telemetry validation.

---

## 🛠️ Step-by-Step Decomposition Protocol

### Step 1: Interface & Contract Specification
Define a single source of truth for communication between components:
- Shared types and interfaces.
- Error codes and payload formats.
- Preconditions and postconditions.

### Step 2: Milestone & Dependency Mapping
Create a topological execution plan:
```
[Phase 1: Contract & DB Schema] (Backend Subagent)
              │
       ┌──────┴──────┐
       ▼             ▼
[Phase 2A: API]  [Phase 2B: UI Mockups] (Frontend Subagent)
       └──────┬──────┘
              ▼
[Phase 3: Integration & E2E Tests] (QA Subagent)
```

### Step 3: Work Package Definition
For each work package, compile an explicit assignment card:
- **Assigned Preset Role**: e.g., `backend`, `web`, `qa`.
- **Target Files**: Explicit file paths the agent has permission to edit.
- **Contract Reference**: Interfaces the implementation must satisfy.
- **Acceptance Criteria**: Concrete commands or unit tests that must pass.
