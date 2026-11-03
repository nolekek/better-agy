---
name: agent-preset-conductor
description: "MANDATORY META-ROUTER v3.0: Evaluates every prompt, switches preset or auto-composes hybrid overlay. Never halts user for multi-domain stacks."
---

# Preset Conductor v3.0

Autonomous semantic router for dynamic context optimization.

## ⚡ Routing Protocol
1. **Eval**: Classify domain from user prompt.
2. **Badge**: Output `🎯 [Смена пресета: <ID>] Причина: <tech>` (or `🎯 [Гибридный пресет: <base> +<overlay>]`).
3. **Switch**: Execute `agy preset switch <base> [+overlay]`.
4. **No-Churn**: Keep preset if tools are already active.

## 🔀 Auto-Hybrid Mode (Zero Friction)
On multi-domain tasks (e.g. Next.js + Playwright, FastAPI + Docker):
- **NEVER** halt or invoke grill-me for combining tech stacks.
- Autonomously compose `base +overlay` (e.g. `web +qa`, `backend +devops`) and proceed immediately.

## 🛡️ Guardrails
- **`vst3`**: ONLY audio DSP, JUCE C++, VST/AU plugins. Never browser or generic C++.
- **`meta-dev`**: ONLY Antigravity repo (`antigravityBUTmultiacc`, `agy`). Never external apps.
- **`orchestrator`**: ONLY multi-agent fan-out (2+ subagents). Never single-domain.
- **`grill-me`**: ONLY when functional goals are contradictory or ambiguous.

## 🧭 Presets
- `web`: Next.js, React, Vue, Tailwind, UI/UX, SEO.
- `backend`: Postgres, SQL, REST/GraphQL, FastAPI, Node, Go, Redis, Kafka.
- `devops`: Docker, K8s, GitHub Actions, Terraform, AWS, SRE.
- `appsec`: OWASP Top-10, pentest, API security, ASAR audit, MITM.
- `qa`: Playwright E2E, Windows-MCP, TDD, test fixing, k6.
- `ai-agents`: MCP, RAG, Prompt Eng, LangGraph, evals.
- `web-research`: Deep search, scraping, OSINT recon.
- `reverse-engineering`: Clean-room replica, Craft Anything, ASAR.
- `product`: Monetization, unit economics, offers, CRO, psychometrics.
- `orchestrator`: Multi-agent DAG, concurrent subagents, synthesis.
- `systems`: C++20/23, C, Zig, Rust, SIMD, lock-free.
- `vst3`: JUCE, C++ real-time DSP, VST3/AU plugins, synths.
- `hardware`: CAD/CAM, SolidCraft 3D, CADCraft DWG, TextureCraft.
- `media`: FilmCraft, EffectCraft VFX, LightCraft, PhotoCraft, SVG.
- `meta-dev`: Antigravity core, Electron ASAR, multiacc CLI.
- `minimal`: Minimal context conversational mode.
