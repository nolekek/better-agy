# Red-Team Assumption Attack

The pre-mortem produces failure stories; the red-team step explains *why each one is
possible* by exposing the belief the plan silently depends on. A failure mode that does not
break a stated assumption is just a worry. A failure mode tied to a load-bearing assumption
is actionable, because you can test or hedge the assumption directly.

## Surface the load-bearing assumptions

List the beliefs the plan must have true to work. They usually hide in confident verbs:
"users **will** share it", "the channel **will** convert at 3%", "we **can** ship by Q3",
"competitors **won't** respond for a year". Mark each as **load-bearing** if the plan
collapses when it is false — those are the ones worth attacking.

## Attack each assumption

For every load-bearing assumption, run three challenges:

- **Evidence**: what do we actually know versus assume? Tag the basis as `observed`,
  `estimated`, or `assumed`. An assumption resting only on `assumed` is fragile.
- **Inversion**: suppose the opposite is true. What breaks, and which failure story does
  that produce? This is how assumptions get linked to specific failure modes.
- **Adversary**: who benefits if this assumption is wrong — a competitor, a platform, a
  skeptical buyer — and would they act to make it wrong?

## Link, do not duplicate

Each top failure mode should name exactly one assumption it breaks (`breaks_assumption`).
This linkage is the payoff: it turns "things might go wrong" into "these specific beliefs
are doing the load and here is how to pressure-test them before betting on them." If a
failure mode breaks no assumption, either it is not real or you have missed an assumption —
add it. The attack stays constructive: the output is a stronger plan with its riskiest
beliefs identified, not a verdict that the plan is doomed.
