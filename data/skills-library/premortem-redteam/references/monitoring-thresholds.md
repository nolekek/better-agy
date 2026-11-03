# Monitoring Thresholds

A ranked failure mode is only useful if you find out it is happening while you can still
act. Each top failure mode therefore gets a **leading indicator** and a concrete
**threshold** — the early-warning system that converts the pre-mortem from a one-time
exercise into something that keeps protecting the plan after launch.

## Leading, not lagging

Pick indicators that move *before* the failure is irreversible. "Revenue missed the annual
target" is a lagging indicator — by the time it trips, the year is gone. "Week-4 activation
rate is below X%" is leading: it predicts the revenue miss while there is still time to
change the onboarding. Tie the indicator to the failure mode's mechanism, not to the final
outcome.

## Make the threshold concrete

A threshold must be unambiguous enough that two people reading the same dashboard would
agree whether it tripped. Specify:

- **Metric** — exactly what is measured (e.g. "paid-channel CAC", "week-1 retention").
- **Trigger value** — the number and direction ("> $80", "< 25%").
- **Window** — over what period and how persistently ("two consecutive weeks", "by day 30").

"Watch engagement" is not a threshold. "If 7-day retention is below 25% for two consecutive
cohorts, the retention failure mode is materializing" is.

## Pair every threshold with a pre-decided response

The value of an early warning is the action it triggers. For each threshold, write the
response *now*, before the pressure of launch: the mitigation to execute, the budget to
pause, or the assumption to re-test. Deciding the response in advance prevents the common
trap of watching a metric cross the line and rationalizing it. The output for each top
failure mode is the quartet: indicator → threshold → response → owner.
