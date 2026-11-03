# Example: fictional launch pre-mortem

A complete, worked pre-mortem red-team on a fully invented product launch. Every name,
number, and channel here is fictional — "Notewise" is not a real product, and nothing in
this folder reflects a real roadmap, customer, or distribution plan. It exists so a reader
can see the whole method produce a shareable report.

## The plan under attack

A three-person team plans to launch **Notewise**, an AI note-taking app for independent
consultants, on a LinkedIn-only acquisition bet with a Q3 launch and a six-month runway.
Target: 500 paying users in twelve months. The full structured analysis lives in
[`analysis.json`](analysis.json).

## What the analysis shows

The plan rests on four assumptions, three of them load-bearing: that LinkedIn content
drives most signups (A1), that trials convert to paid at $20/mo (A2), and that v1 ships on
time (A3). Each failure mode is tied to the assumption it breaks, then scored on
likelihood × impact:

- **F1 — LinkedIn channel under-delivers** (4 × 5 = 20): the single-channel funnel is the
  biggest risk; mitigation is a content buffer plus a second channel lined up *before* launch.
- **F2 — trials do not convert** (3 × 5 = 15): gate scale-up on a defined week-1 activation event.
- **F3 — v1 slips past Q3** (3 × 4 = 12): freeze a minimal launch scope tied to activation.
- **F4 — incumbent ships comparable AI** (2 × 3 = 6): logged lower; lean on a consultant-specific moat.

Each top failure mode carries a leading indicator, a concrete threshold, a pre-decided
response, and an owner — so the pre-mortem keeps protecting the plan after launch, not just
on the day it was written. The residual risk (over-reliance on one founder's content
output) is named and accepted knowingly rather than hidden.

## Reproduce the report

```bash
python3 scripts/generate_html_report.py \
    --input examples/fictional-launch-premortem/analysis.json \
    --output examples/fictional-launch-premortem/report.html
```

The standalone HTML is checked in as [`report.html`](report.html): ranked failure-mode
table (priority-banded), a monitoring-and-mitigation table, and the residual-risk note.
