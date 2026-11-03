---
name: multi-agent-debate-consensus
description: "Facilitates structured multi-agent deliberation, Maker-Checker validation loops, Red Team/Blue Team audits, and architectural consensus arbitration to catch flaws and prevent hallucinations in complex systems."
risk: low
source: antigravity-orchestrator-core (Apache 2.0)
date_added: 2026-10-31
---

# Multi-Agent Debate & Consensus

The **Multi-Agent Debate & Consensus** skill enforces rigor and quality in multi-agent architectures by pairing generative agents with adversarial or auditing agents before critical decisions or code merges take place.

---

## ⚖️ Collaboration Topologies

### 1. Maker-Checker (Generator / Verifier)
- **Generator (Builder Subagent)**: Implements the feature or writes the solution.
- **Checker (Auditor / QA Subagent)**: Independently examines the code against edge cases, security vulnerabilities, performance regressions, and test failures.
- **Rule**: Generator cannot self-approve; Checker must issue a clean sign-off.

### 2. Red Team / Blue Team (Security & Resilience)
- **Blue Team (Architect / Developer)**: Proposes design, API structure, authentication flows.
- **Red Team (AppSec / Reverse Engineer Subagent)**: Actively attacks the proposal, seeking injection points, auth bypasses, race conditions, or unhandled exceptions.
- **Resolution**: Blue Team patches findings before deployment.

### 3. Dialectical Debate & Consensus
When two architectural paths exist (e.g., REST vs WebSocket, monolithic vs microservice):
- Subagent A defends Option 1 with trade-offs.
- Subagent B defends Option 2 with trade-offs.
- Orchestrator serves as the impartial arbiter, scoring proposals against project constraints (latency, complexity, maintenance appetite).

---

## 📋 Review Criteria Checklist for Checker Subagents

1. **Contract Adherence**: Does the implementation strictly conform to the defined interface?
2. **Failure Modes**: Are network disconnects, timeouts, and empty inputs gracefully handled?
3. **Workspace Hygiene**: Were any temporary, scrap, or disposable files created in the workspace root?
4. **Test Proof**: Are there passing unit or integration tests proving correctness?
5. **Security Check**: Are credentials protected, inputs sanitized, and least-privilege principles respected?
