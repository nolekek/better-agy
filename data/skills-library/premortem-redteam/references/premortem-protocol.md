# Pre-mortem Protocol

The pre-mortem inverts the usual planning question. Instead of asking "how do we make this
succeed?", it asserts the failure as a fact and works backward: *it is twelve months from
now, the plan has clearly failed — write the story of why.* This framing defeats the
optimism that normally suppresses risk talk, because the failure is no longer hypothetical
to argue against; it is a given to explain.

## Intake

Capture the plan before attacking it. Record:

- **Title and summary** — what is being launched, decided, or committed to.
- **The decision** — the specific commitment the plan represents (ship date, budget, channel bet).
- **The horizon** — the point in the future at which we judge success or failure (e.g. 12 months).

A vague plan produces vague failure modes. If the plan has no measurable decision or
horizon, sharpen those first.

## Generating failure stories

Run several passes, each from a different vantage so the failures are not all of one kind:

- **Demand**: nobody wanted it, or not enough did, at the price assumed.
- **Execution**: the team could not build or operate it on time or quality.
- **Distribution**: the channel that was supposed to deliver reach did not.
- **Economics**: unit economics or runway broke before traction arrived.
- **External**: a competitor, regulator, platform change, or macro shift intervened.

For each pass, write a concrete one-paragraph story in past tense ("by month nine, …").
Concreteness matters: "marketing failed" is not a failure mode; "the LinkedIn channel that
was supposed to drive 60% of signups produced almost none because the founder's posting
cadence collapsed after launch crunch" is. Each story becomes a candidate failure mode that
the red-team step then ties to a load-bearing assumption.
