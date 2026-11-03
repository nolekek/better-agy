# Output Contract

Defines what a finished pre-mortem red-team hands back. The deliverable is a decision aid
that makes the plan stronger — a ranked, monitored, mitigated set of failure modes — not a
generic risk list and not a verdict that the plan is doomed.

## Structured analysis (source of truth)

Capture the analysis as a JSON object so it is reproducible and renderable. The shape used
by `scripts/generate_html_report.py`:

- `plan` — `title`, `summary`, `decision`, `horizon`.
- `assumptions[]` — `id`, `statement`, `load_bearing`.
- `failure_modes[]` — `id`, `title`, `story`, `breaks_assumption` (an assumption id),
  `likelihood` (1–5), `impact` (1–5), `indicator`, `threshold`, `response`, `mitigation`,
  `owner`.
- `residual_risk` — the risk the team accepts knowingly.

The report computes `priority = likelihood × impact` and ranks descending.

## Rendered report

The report (Markdown or the standalone HTML from the script) must contain:

1. **Plan and decision** — what is being committed to, and the horizon it is judged against.
2. **Load-bearing assumptions** — the beliefs the plan depends on, flagged.
3. **Ranked failure-mode table** — title, the assumption it breaks, likelihood × impact =
   priority, sorted so the most dangerous failures are first.
4. **Monitoring table** — for each top failure mode: leading indicator, concrete threshold,
   pre-decided response, and owner.
5. **Mitigation packages** — the change that lowers each top failure mode's likelihood or impact.
6. **Residual risk** — what remains after mitigation, accepted on purpose.

## Rules

- Every top failure mode names exactly one assumption it breaks.
- Every top failure mode carries an indicator, a concrete threshold, and a response.
- Be adversarial in finding failures, constructive in the hand-back: end with a stronger plan.
- Pure method, no local or private data; published examples use a fictional plan only.

## Local Run Artifacts

When generating a standalone HTML report, write it into the ignored skill-local output folder:

```text
skills/premortem-redteam/output/
  YYYY-MM-DD-premortem-report.html
```

Use `--run-id` when multiple reports are generated on the same day. Do not commit `output/` contents.
