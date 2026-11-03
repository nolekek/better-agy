# Marty Cagan's 4 Big Product Risks: Assessment Rubric

Before writing any substantial code, every feature and product concept should be evaluated against Marty Cagan's 4 Big Product Risks. Use this rubric during **Gateway 1** to stress-test ideas.

---

## 1. Value Risk
> *Will customers buy this or choose to use it?*

### Key Questions to Ask:
- Does this solve an active, acute pain point, or is it merely "nice to have"?
- How are target users currently solving this problem? Is our solution 10x better, faster, or cheaper?
- Why will users switch from their existing habit or tool?
- What happens if we do *not* build this? Will anyone complain or churn?

### Red Flags / Anti-Patterns:
- *"Users will love this once they see it."* (Speculative value).
- *"Competitor X has this button, so we need it too."* (Cargo cult feature parity).
- Solving problems that only occur once a year or to 0.1% of users without strategic justification.

### How to Mitigate:
- Create a low-fidelity prototype or mock and show it to target users.
- Check search query volume, customer support tickets, or user interview quotes.
- Run a fake-door test (Call-to-Action leading to a waitlist modal).

---

## 2. Usability Risk
> *Can users figure out how to use it without instructions?*

### Key Questions to Ask:
- What is the Time-to-First-Value (TTFV)? Can a user achieve their primary goal in under 60 seconds?
- Is the mental model aligned with user expectations, or does it require learning new terminology?
- Does the interface guide the user naturally through visual hierarchy, or is it crowded with competing buttons?
- How obvious are the empty states and next steps?

### Red Flags / Anti-Patterns:
- Requiring a multi-page user manual or mandatory 5-step tutorial before the product is usable.
- Ambiguous icon buttons without labels or tooltips.
- Hidden critical actions inside nested multi-level dropdown menus.

### How to Mitigate:
- Follow established UI design patterns (e.g., standard search bars, breadcrumbs, consistent keyboard shortcuts).
- Provide immediate, contextual feedback for every user interaction.
- Test with someone unfamiliar with the project and observe where they get stuck without intervening.

---

## 3. Feasibility Risk
> *Can our engineers build this with our available time, skills, and technology?*

### Key Questions to Ask:
- Do we have the technical capabilities, libraries, and infrastructure to deliver this reliably?
- What are the external dependencies (third-party APIs, LLM rate limits, SDK stability)?
- How does this scale under realistic loads, concurrent requests, or large datasets?
- Are there hidden performance bottlenecks (e.g., synchronous blocking calls, N+1 database queries)?

### Red Flags / Anti-Patterns:
- Relying on undocumented or fragile third-party APIs with unclear SLAs.
- Choosing complex distributed systems (microservices, Kafka, distributed locks) for early-stage MVPs.
- Underestimating edge-case latency (e.g., multi-second LLM streaming without streaming UI).

### How to Mitigate:
- Build a rapid, disposable spike/prototype to test the highest-risk technical assumption in under 2 hours.
- Prefer boring, battle-tested technologies for the core foundation.
- Implement progressive degradation (fallbacks when advanced features fail).

---

## 4. Business Viability Risk
> *Does this solution work for our business model, legal, compliance, and operational constraints?*

### Key Questions to Ask:
- **Cost**: Does the unit economics make sense? (e.g., LLM inference tokens vs user subscription price).
- **Compliance & Privacy**: Does this touch GDPR, HIPAA, PII, or proprietary customer data?
- **Support & Maintenance**: Will this create an unbearable customer support burden?
- **Go-to-Market**: Do we have a credible channel to distribute this feature to users?

### Red Flags / Anti-Patterns:
- High variable costs per query that exceed revenue or budget.
- Collecting sensitive user data without clear retention, encryption, or deletion policies.
- Features that can be easily abused (e.g., spam generation, open relays, SSRF vulnerabilities).

### How to Mitigate:
- Calculate cost-per-user / cost-per-action upfront based on expected usage volume.
- Implement rate limits, quotas, and authorization boundaries from day one.
- Involve compliance/legal considerations early in the architecture phase.
