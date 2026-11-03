# Product Requirements Document: Smart CLI Snippet Manager (SnippetSync)

---

## 1. Document Overview

| Field | Value |
| :--- | :--- |
| **Product / Feature Name** | SnippetSync CLI |
| **Owner / Author** | Product Engineering Team |
| **Status** | Approved |
| **Target Appetite / Timebox** | 1 Week (Shape Up) |
| **Target Release / Version** | v0.1.0-alpha |

---

## 2. Executive Summary & Problem Framing

### 2.1 The Problem Statement
- **What is the problem?**: Developers frequently reuse complex shell one-liners, Docker commands, and curl requests, but lose them in sprawling bash history or unorganized markdown files.
- **Who suffers from it?**: Software engineers, DevOps/SREs, and CLI-heavy developers.
- **Current Workarounds**: Grepping `~/.bash_history` (messy, transient), pasting snippets in private Slack channels, or maintaining personal Notion/Gist pages (slow context switch from terminal).

### 2.2 Jobs to be Done (JTBD)
> *"When I need a complex terminal command I ran last week, I want to search and execute it directly from my CLI in seconds, so I can stay in flow without breaking focus to search external browser tabs."*

### 2.3 Why Now?
CLI workflows are increasingly central to modern AI-assisted engineering; friction in terminal command recall directly hurts daily developer velocity.

---

## 3. Target Audience & Personas

- **Primary Persona**: Alex (Senior Full-Stack Developer), works 8+ hours/day in terminal, values keyboard-first speed, hates slow web dashboards.
- **Anti-Persona (Out of Scope)**: Non-technical casual computer users who need a graphical desktop UI or mobile app.

---

## 4. The 4 Product Risks Evaluation (Marty Cagan)

1. **Value Risk**: High risk of being ignored if search is slower than `Ctrl+R`. Mitigated by fuzzy search latency < 50ms and instant clipboard copy.
2. **Usability Risk**: Risk of complex command syntax. Mitigated by simple 3-verb CLI: `snip add`, `snip find`, `snip run`.
3. **Feasibility Risk**: Multi-OS shell differences (zsh, bash, fish, PowerShell). Mitigated by storing snippets as plain text and executing via subshell.
4. **Viability Risk**: Zero server infrastructure costs for MVP (local SQLite file `~/.snippetsync.db`).

---

## 5. Scope Boundaries & Appetite (Shape Up Style)

### 5.1 Fixed Appetite
- **Timebox**: 1 week sprint. Scope will be cut if blocked; timebox will not be extended.

### 5.2 IN-SCOPE (Must-Haves for MVP)
- [x] CLI command `snip add "<title>" "<command>" [-t tag]`
- [x] Interactive fuzzy finder `snip find` with preview and Enter to copy to clipboard
- [x] Local SQLite storage with auto-migration
- [x] Export/Import to standard JSON for local backup

### 5.3 OUT-OF-SCOPE (Explicitly Deferred)
- Cloud synchronization and team sharing (Phase 2).
- AI auto-generation of commands from natural language (Phase 2).
- Web dashboard or browser extension.
- Integration with third-party password managers for secret storage.

---

## 6. User Journeys & State Architecture

### 6.1 Critical User Flow (Happy Path)
1. **Entry**: User types `snip find docker` in terminal.
2. **Action**: Interactive terminal menu filters snippets in real-time.
3. **Resolution**: User hits `Enter`; snippet is copied to clipboard and displayed in terminal ready to run.

### 6.2 State Matrix

| State | User Experience & Visual Behavior | Recovery / Next Action |
| :--- | :--- | :--- |
| **Empty State** | Running `snip find` with 0 snippets displays: *"No snippets saved yet. Run `snip add 'List files' 'ls -la'` to get started!"* | Clear suggestion with executable example command. |
| **Loading State** | Instant for local database (<20ms). No visible loading screen needed. | N/A |
| **Error State** | If DB file is locked or corrupted: *"Unable to open ~/.snippetsync.db (Permission denied). Try running with correct user permissions."* | Actionable error message with resolution advice. |
| **Edge Cases** | Terminal resized to < 40 columns: gracefully degrades to non-interactive list mode. | Fallback display mode. |

---

## 7. Functional Requirements & Acceptance Criteria

### Requirement 1: Add Snippet
- **Given**: A user in a terminal session.
- **When**: Running `snip add "Deploy Staging" "git push origin main && deploy-script" -t devops`.
- **Then**: Record is persisted to local DB and output prints `✓ Saved snippet #14 ('Deploy Staging')`.

### Requirement 2: Search & Select
- **Given**: At least one saved snippet exists.
- **When**: Running `snip find`.
- **Then**: Launches an interactive fuzzy-finder displaying titles, tags, and command previews.

---

## 8. Non-Functional Requirements

- **Performance**: Startup time to interactive fuzzy menu < 80ms.
- **Security**: Snippets are stored locally with user-only file read/write permissions (`0600`).
- **Reliability**: Graceful handling of SIGINT (`Ctrl+C`) restoring terminal cursor and state.

---

## 9. Telemetry & Success Metrics

### 9.1 Core Success Metrics
- **North Star Metric**: Weekly Active Users executing >= 5 snippet lookups/week.
- **Activation Metric**: % of new installs that save their first snippet within 5 minutes (>70% target).

### 9.2 Event Tracking Schema

| Event Name | Trigger | Properties / Payload |
| :--- | :--- | :--- |
| `snippet_added` | Successful `snip add` | `has_tags: bool`, `command_length: int` |
| `snippet_found` | User copies/executes snippet from `snip find` | `search_query_length: int`, `selection_index: int` |
| `cli_error_encountered` | CLI errors out | `error_type: string`, `command_name: string` |

---

## 10. Open Questions & Risks Register

| # | Question / Assumption | Owner | Resolution / Status |
| :- | :--- | :--- | :--- |
| 1 | Should `snip run` execute commands automatically or copy to clipboard? | Product Lead | Resolved: Copy to clipboard by default; require `--exec` flag to execute directly to prevent accidental damage. |
