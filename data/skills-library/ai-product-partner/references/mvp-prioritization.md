# MVP Scoping & Prioritization: Shape Up, MoSCoW & Anti-Bloat

Overengineering and scope creep are the primary killers of software products and developer velocity. This guide provides actionable frameworks to aggressively trim scope, establish firm boundaries, and ship high-impact MVPs fast.

---

## 1. Shape Up Appetite vs Estimates

In traditional software development, teams ask: *"How long will it take to build this feature?"*
This leads to bloated estimates and scope expansion.

In **Shape Up**, we flip the question:
> **"How much time and energy are we willing to spend solving this problem?"**

### The Appetite Principle:
- **Set a Fixed Appetite**: (e.g., 2 hours for a script, 1 day for a CLI command, 1 week for a feature module).
- **Variable Scope**: The deadline is fixed; the scope is flexible. If time runs out, cut scope—never extend the deadline.
- **The "Circuit Breaker"**: If a feature cannot be completed within its agreed appetite, do not default to extending it. Step back, ask what blocked it, simplify the solution, or discard it.

---

## 2. MoSCoW Prioritization Framework

Classify every candidate requirement into one of four buckets:

| Category | Definition | Rule of Thumb |
| :--- | :--- | :--- |
| **Must Have (M)** | Critical to core value delivery. Without this, the feature or product is fundamentally broken or useless. | Must not exceed 60% of total team capacity. |
| **Should Have (S)** | High value and important, but there is a reasonable workaround or manual alternative for the first release. | Build only if Must-Haves finish early. |
| **Could Have (C)** | Desirable "delight" improvements or minor conveniences. Low risk if omitted. | First candidates to be cut if time is tight. |
| **Won't Have This Time (W)** | Explicitly agreed NOT to be built in this cycle. Stored in backlog for future consideration. | Critical for focus; prevents debates during development. |

---

## 3. ICE & RICE Scoring (For Feature Comparison)

When multiple features compete for priority:

### ICE Scoring (Quick & Intuitive):
$$\text{Score} = \text{Impact} \times \text{Confidence} \times \text{Ease}$$
- **Impact (1-10)**: How much will this improve the target metric or user value?
- **Confidence (1-10)**: How certain are we about our assumptions and problem validation?
- **Ease (1-10)**: How easy/fast is it to implement? (10 = trivial, 1 = months of work).

### RICE Scoring (For Data-Driven Roadmaps):
$$\text{Score} = \frac{\text{Reach} \times \text{Impact} \times \text{Confidence}}{\text{Effort}}$$
- **Reach**: Number of users affected per time period.
- **Impact**: Massive (3x), High (2x), Medium (1x), Low (0.5x), Minimal (0.25x).
- **Confidence**: 100% (High data), 80% (Medium), 50% (Low/Hunch).
- **Effort**: Person-weeks or person-days.

---

## 4. The Anti-Overengineering Checklist

Before approving any architecture or tech plan, run this filter:

| Overengineering Pattern | Symptom | Pragmatic 80/20 Alternative |
| :--- | :--- | :--- |
| **Speculative Generality** | *"Let's build a plugin system and abstract factory in case we support 10 providers later."* | Build for the 1 concrete provider needed today. Refactor when the second provider actually arrives. |
| **Infrastructure Inflation** | Adding Redis, Kafka, or microservices for a system with < 1,000 daily users. | In-memory cache, simple SQLite/PostgreSQL, or background worker threads. |
| **Premature Multi-Tenancy** | Designing complex tenant isolation, organizations, and permissions before having 5 active users. | Simple user ownership column (`user_id`). |
| **Custom Auth/Billing from scratch** | Writing custom JWT refresh rotation, password reset flows, and Stripe webhook routers. | Use standard libraries (e.g. Supabase Auth, NextAuth, Stripe Checkout, Clerk). |
| **Exotic Tech Stack** | Choosing an unproven framework or esoteric database because it is trendy. | Stick to boring, well-documented technologies where AI and engineers are 10x faster. |

---

## 5. Scope-Trimming Tactics: The "Good Enough" Test

When facing an oversized feature, apply these reduction tactics:

1. **Remove Settings & Preferences**: Don't make users choose between 5 options; make one opinionated, great default choice.
2. **Replace Automation with Manual Workarounds**: In v1, can an admin manually trigger a script instead of building a complex automated cron dashboard?
3. **Narrow the Input Constraints**: Support only one file format (e.g., Markdown only) instead of 10 formats (PDF, DOCX, EPUB, RTF) in v1.
4. **Text Over Graphics**: Can a clear textual summary convey the same value as an interactive 3D chart? Start with text.
