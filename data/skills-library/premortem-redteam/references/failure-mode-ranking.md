# Failure-Mode Ranking

A pre-mortem typically surfaces more failure modes than a team can act on. Ranking forces
prioritization so attention and mitigation budget go to the failures that actually threaten
the plan, not the most vivid or most recently mentioned ones.

## The two axes

Score each failure mode on two 1–5 scales:

- **Likelihood** — how probable is this failure over the plan's horizon? 1 = remote,
  5 = expected unless something changes. Anchor the score to the assumption it breaks: a
  failure resting on a purely `assumed` belief usually scores higher than one resting on
  `observed` evidence.
- **Impact** — if it happens, how badly does it hurt the decision? 1 = a setback you absorb,
  5 = the plan fails outright or burns the runway.

## The priority score

Multiply: `priority = likelihood × impact`, giving a 1–25 range. Sort descending. The top
band (roughly 15+) is where mitigation and monitoring are mandatory; the middle band
(8–14) gets a watch indicator but lighter mitigation; the low band (under 8) is logged as
residual risk and accepted knowingly.

## Discipline rules

- Score honestly, not defensively. The point is to find the failures worth preventing, so
  resist deflating likelihood on the plan you are emotionally invested in.
- Keep likelihood and impact independent. A high-impact, low-likelihood "black swan" and a
  low-impact, high-likelihood "papercut" are different decisions; multiplying keeps them
  comparable without collapsing them.
- Break ties with reversibility: between two equal scores, the harder-to-reverse failure
  ranks higher, because monitoring buys less time when recovery is slow.
- Re-rank after mitigation. A strong mitigation lowers likelihood or impact, which can move
  a failure mode out of the mandatory band — record both the pre- and post-mitigation score
  so the board shows what the plan changes bought.
