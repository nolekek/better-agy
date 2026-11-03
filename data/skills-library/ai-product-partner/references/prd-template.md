# Product Requirements Document (PRD) Template

> **Instructions**: Use this template to document features or new products during Gateway 4. Keep descriptions concrete, concise, and focused on user value.

---

## 1. Document Overview

| Field | Value |
| :--- | :--- |
| **Product / Feature Name** | `[Feature / Product Name]` |
| **Owner / Author** | `[Author / Team]` |
| **Status** | `[Draft | Review | Approved | In Development | Shipped]` |
| **Target Appetite / Timebox** | `[e.g., 2 days | 1 week | 4 weeks (Shape Up)]` |
| **Target Release / Version** | `[e.g., v0.1.0-alpha | v1.2.0]` |

---

## 2. Executive Summary & Problem Framing

### 2.1 The Problem Statement
- **What is the problem?**: `[Describe the specific pain point or friction users face.]`
- **Who suffers from it?**: `[Target user segment or role.]`
- **Current Workarounds**: `[How do users solve this today? What is clumsy, slow, or expensive about it?]`

### 2.2 Jobs to be Done (JTBD)
> *"When `[situation/trigger]`, I want to `[action/motivation]`, so I can `[desired outcome/benefit]`."*

### 2.3 Why Now?
`[Why is this the right time to build this? Strategic alignment, customer requests, market trend, or technical unlock.]`

---

## 3. Target Audience & Personas

- **Primary Persona**: `[Name/Role, context, technical proficiency, key goals.]`
- **Secondary Persona**: `[Occasional users, admins, or stakeholders.]`
- **Anti-Persona (Who this is NOT for)**: `[Explicitly list users we are not optimizing for in this release to prevent scope bloat.]`

---

## 4. The 4 Product Risks Evaluation (Marty Cagan)

1. **Value Risk**: `[Why will users choose this over doing nothing or using alternatives? Evidence?]`
2. **Usability Risk**: `[Where might users get confused? How is TTFV minimized?]`
3. **Feasibility Risk**: `[What are the toughest technical hurdles, third-party limits, or performance bottlenecks?]`
4. **Viability Risk**: `[Unit economics, API token costs, security, privacy/GDPR, or compliance impact.]`

---

## 5. Scope Boundaries & Appetite (Shape Up Style)

### 5.1 Fixed Appetite
- **Timebox**: `[e.g., 3 days]`
- **Philosophy**: Fix time and quality; flex scope.

### 5.2 IN-SCOPE (Must-Haves for MVP)
- `[ ]` `[Core capability 1 - absolute minimum needed for value delivery]`
- `[ ]` `[Core capability 2]`
- `[ ]` `[Core capability 3]`

### 5.3 OUT-OF-SCOPE (Explicitly Deferred to Future Iterations)
> *Listing what we are NOT building is as important as listing what we are building.*
- `[Deferred item 1 - e.g., complex role-based access control]`
- `[Deferred item 2 - e.g., third-party export to 10 platforms]`
- `[Deferred item 3 - e.g., custom theming and animations]`

---

## 6. User Journeys & State Architecture

### 6.1 Critical User Flow (Happy Path)
1. **Entry**: `[User opens page / runs command / clicks button]`
2. **Action**: `[User inputs data / selects option]`
3. **Feedback**: `[System immediately acknowledges / shows progress]`
4. **Resolution**: `[Value delivered / output presented]`

### 6.2 State Matrix

| State | User Experience & Visual Behavior | Recovery / Next Action |
| :--- | :--- | :--- |
| **Empty State** | `[No items yet. Display helpful illustration, sample template, or 1-click starter.]` | `[CTA button: 'Create your first X']` |
| **Loading State** | `[Skeleton screens, progress percentage, or responsive spinner. Avoid UI lockup.]` | `[Cancel option if > 5s]` |
| **Error State** | `[Human-readable error explanation: what broke and why, no raw stack traces.]` | `[Retry button / 'Edit inputs' link]` |
| **Edge Cases** | `[Network disconnect, duplicate item name, max size limit exceeded, timeout.]` | `[Graceful degradation / local cache]` |

---

## 7. Functional Requirements & Acceptance Criteria

### Requirement 1: `[Capability Name]`
- **Description**: `[What the system must do]`
- **Acceptance Criteria (Gherkin format)**:
  - **Given**: `[precondition]`
  - **When**: `[action taken]`
  - **Then**: `[expected observable result]`

### Requirement 2: `[Capability Name]`
- **Description**: `[What the system must do]`
- **Acceptance Criteria**:
  - **Given**: `[precondition]`
  - **When**: `[action taken]`
  - **Then**: `[expected observable result]`

---

## 8. Non-Functional Requirements

- **Performance**: `[e.g., P95 response time < 500ms; initial page load < 1.5s]`
- **Security & Privacy**: `[e.g., Auth required, no secrets in logs, sanitized input]`
- **Reliability & Resilience**: `[e.g., Graceful error handling, auto-retry on 5xx with backoff]`
- **Accessibility**: `[e.g., Keyboard navigation, WCAG AA contrast]`

---

## 9. Telemetry & Success Metrics

### 9.1 Core Success Metrics
- **North Star Metric**: `[e.g., Weekly Active Workspaces generating at least 1 report]`
- **Activation Metric**: `[e.g., % of new signups reaching 'Aha!' moment within first 10 minutes]`
- **Retention / Quality Metric**: `[e.g., Day-7 retention rate, task completion rate > 90%]`

### 9.2 Event Tracking Schema

| Event Name | Trigger | Properties / Payload |
| :--- | :--- | :--- |
| `[object]_[action]` (e.g. `report_created`) | `[When user clicks 'Generate' and succeeds]` | `user_id`, `template_type`, `duration_ms` |
| `[object]_[action]` (e.g. `report_failed`) | `[When generation errors out]` | `user_id`, `error_code`, `step` |

---

## 10. Open Questions & Risks Register

| # | Question / Assumption | Owner | Resolution / Status |
| :- | :--- | :--- | :--- |
| 1 | `[e.g., Will the free tier API limits suffice for 100 concurrent users?]` | `Tech Lead` | `[Open / Investigating]` |
| 2 | `[e.g., Do users prefer inline editing or a modal window?]` | `Designer` | `[Resolved: Inline]` |
