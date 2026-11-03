---
name: premortem-redteam
description: Stress-test a plan or decision with a structured pre-mortem and red-team rebuttal before committing. Use when the user has a plan, launch, strategy, or decision and wants to surface how it could fail, which assumptions it depends on, what early-warning indicators to watch, and how to mitigate. Produces ranked failure modes, monitoring thresholds, and mitigation packages. Do not use for executing the plan, generic brainstorming with no plan to attack, or final licensed legal/financial advice.
---

# Pre-mortem Red Team

## Use This Skill For

- run a pre-mortem: assume the plan failed 12 months out, then work backward to why
- red-team the plan's core assumptions — attack the reasoning, not just the execution
- rank failure modes by likelihood × impact
- define leading indicators / monitoring thresholds that would warn early
- attach a mitigation package to each top failure mode
- optionally use the Decision-Grade Research search adapter to check external facts behind a high-risk assumption

A method skill on the same shelf as crux and game-theory: zero private data, pure framework.

## Do Not Route Here

- executing or building the plan
- generic brainstorming with no concrete plan or decision to attack
- final licensed legal, financial, or medical advice

## Default Workflow

1. Use `references/premortem-protocol.md` to intake the plan and generate failure scenarios from a "it already failed — why?" stance.
2. Use `references/redteam-assumptions.md` to list the plan's load-bearing assumptions and attack each one.
3. Use `references/failure-mode-ranking.md` to score failure modes by likelihood and impact and rank them.
4. Use `references/monitoring-thresholds.md` to define the leading indicator and threshold that would warn before each top failure mode lands.
5. Pair each top failure mode with a mitigation in `references/output-contract.md`.
6. For fact-sensitive assumptions, optionally run `../decision-grade-research/scripts/search_sources.py` and record only the evidence that survives source review.
7. Run `scripts/generate_html_report.py` when the user wants a polished standalone report.

## Core Rules

- start from failure, not success: the pre-mortem assumes the plan has already failed
- attack assumptions explicitly — every top failure mode names the assumption it breaks
- rank, don't dump: prioritize failure modes by likelihood × impact
- every top failure mode gets a leading indicator with a concrete threshold, not a vague "watch this"
- be adversarial but constructive: the output is a stronger plan, not a verdict that it is doomed
- pure method by default; external search is optional and only supports assumption checks
- never hard-code API keys; public examples use a fictional plan

## Output Contract

- deliver a pre-mortem + red-team report, not a generic risk list
- include:
  - the plan and the decision it represents
  - ranked failure modes (likelihood × impact)
  - the load-bearing assumption each top failure mode breaks
  - leading indicator and monitoring threshold per top failure mode
  - mitigation package per top failure mode
  - residual risk the user should accept knowingly
