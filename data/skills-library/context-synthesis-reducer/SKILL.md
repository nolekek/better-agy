---
name: context-synthesis-reducer
description: "Applies Map-Reduce context compression and synthesis strategies to condense extensive subagent outputs, test traces, and git diffs into high-density executive briefings, eliminating context rot in orchestrator sessions."
risk: low
source: antigravity-orchestrator-core (Apache 2.0)
date_added: 2026-10-31
---

# Context Synthesis & Reducer

The **Context Synthesis & Reducer** skill prevents context window exhaustion, attention degradation, and context rot when managing multiple concurrent or sequential subagents.

---

## 📉 Context Inflation vs. Synthesis

In a multi-agent system, subagents generate voluminous raw outputs: complete file contents, detailed build and test traces, raw diffs, and debugging logs. Feeding raw transcripts into the orchestrator context quickly leads to context bloat and lost instructions.

The **Reducer** pattern condenses subagent outputs into structured, high-signal tokens before they reach the orchestrator.

---

## 🗜️ Reduction Strategies

### 1. Extract-and-Compress (Artifact-First)
- Instruct subagents to write detailed reports, logs, and benchmark tables into dedicated markdown artifacts in `<appDataDir>\brain\<conversation-id>\`.
- The subagent returns only a 3-5 line executive summary + artifact link to the orchestrator:
  - **Status**: Completed / Failed / Blocked.
  - **Key Changes**: 2-3 bullet points of what was added or modified.
  - **Verification Evidence**: Test pass count (e.g., `✔ 12/12 tests passed`).
  - **Artifact Reference**: Path to the detailed report.

### 2. Diff Distillation
- Never pipe full 500-line diffs across agent boundaries.
- Extract file paths modified, exported symbol changes, and signature diffs.

### 3. Error Root-Cause Extraction
- If a subagent fails, strip repetitive stack traces.
- Synthesize the failure into:
  - **Failing Operation**: Exact command or API call.
  - **Root Cause**: The underlying assertion failure or error code.
  - **Recommended Action**: Proposed fix or needed clarification.

---

## 📌 Standard Reducer Template for Subagent Handover

```markdown
### 📢 [Этап завершен: {Subagent Name}]
- **Статус**: ✔ Успешно / ⚠ Требует внимания
- **Измененные модули**: `path/to/file1.ts`, `path/to/file2.ts`
- **Верификация**: Все тесты пройдены (`X/X passed in Ys`)
- **Ключевой результат**: {Краткое описание созданной функциональности}
- **Подробный отчет**: [Детальный отчет](file:///path/to/artifact.md)
```
