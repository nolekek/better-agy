#!/usr/bin/env python3
"""Render a pre-mortem red-team analysis (JSON) into a standalone HTML report.

Reads the structured analysis defined in references/output-contract.md from --input or
stdin, computes each failure mode's priority (likelihood x impact), ranks them, and writes
a single self-contained HTML file (no external assets) suitable for sharing.

Example:
    python3 generate_html_report.py --input analysis.json --output-dir output --run-id 2026-06-04
"""

import argparse
import html
import json
import sys
from datetime import date
from pathlib import Path

STYLE = """
body { font-family: -apple-system, Segoe UI, Roboto, Helvetica, Arial, sans-serif;
       max-width: 900px; margin: 2rem auto; padding: 0 1rem; color: #1a1a1a; line-height: 1.5; }
h1 { border-bottom: 3px solid #c0392b; padding-bottom: .3rem; }
h2 { margin-top: 2rem; color: #c0392b; }
table { border-collapse: collapse; width: 100%; margin: 1rem 0; }
th, td { border: 1px solid #ddd; padding: .5rem .6rem; text-align: left; vertical-align: top; }
th { background: #f7f2f2; }
.prio { font-weight: 700; text-align: center; }
.hi { background: #f8d7da; } .mid { background: #fff3cd; } .lo { background: #e9ecef; }
.tag { font-size: .75rem; color: #fff; background: #c0392b; border-radius: 3px; padding: 1px 6px; }
.muted { color: #666; font-size: .9rem; }
"""


def band(priority: int) -> str:
    if priority >= 15:
        return "hi"
    if priority >= 8:
        return "mid"
    return "lo"


def esc(value) -> str:
    return html.escape(str(value if value is not None else ""))


def render(data: dict) -> str:
    plan = data.get("plan", {})
    assumptions = data.get("assumptions", [])
    fms = []
    for fm in data.get("failure_modes", []):
        fm = dict(fm)
        fm["priority"] = int(fm.get("likelihood", 0)) * int(fm.get("impact", 0))
        fms.append(fm)
    fms.sort(key=lambda f: f["priority"], reverse=True)

    out = ["<!doctype html><html lang='en'><head><meta charset='utf-8'>",
           f"<title>Pre-mortem: {esc(plan.get('title', 'Plan'))}</title>",
           f"<style>{STYLE}</style></head><body>"]

    out.append(f"<h1>Pre-mortem Red Team — {esc(plan.get('title', 'Plan'))}</h1>")
    if plan.get("summary"):
        out.append(f"<p>{esc(plan['summary'])}</p>")
    out.append("<p class='muted'>")
    if plan.get("decision"):
        out.append(f"<strong>Decision:</strong> {esc(plan['decision'])} &nbsp; ")
    if plan.get("horizon"):
        out.append(f"<strong>Horizon:</strong> {esc(plan['horizon'])}")
    out.append("</p>")

    if assumptions:
        out.append("<h2>Load-bearing assumptions</h2><ul>")
        for a in assumptions:
            tag = " <span class='tag'>load-bearing</span>" if a.get("load_bearing") else ""
            out.append(f"<li><strong>{esc(a.get('id'))}</strong>: {esc(a.get('statement'))}{tag}</li>")
        out.append("</ul>")

    out.append("<h2>Ranked failure modes</h2>")
    out.append("<table><tr><th>#</th><th>Failure mode</th><th>Breaks</th>"
               "<th>L</th><th>I</th><th>Priority</th></tr>")
    for i, fm in enumerate(fms, 1):
        out.append(
            f"<tr class='{band(fm['priority'])}'><td>{i}</td>"
            f"<td><strong>{esc(fm.get('title'))}</strong><br><span class='muted'>{esc(fm.get('story'))}</span></td>"
            f"<td>{esc(fm.get('breaks_assumption'))}</td>"
            f"<td>{esc(fm.get('likelihood'))}</td><td>{esc(fm.get('impact'))}</td>"
            f"<td class='prio'>{fm['priority']}</td></tr>")
    out.append("</table>")

    out.append("<h2>Monitoring &amp; mitigation</h2>")
    out.append("<table><tr><th>Failure mode</th><th>Leading indicator</th>"
               "<th>Threshold</th><th>Response</th><th>Mitigation</th><th>Owner</th></tr>")
    for fm in fms:
        out.append(
            f"<tr><td><strong>{esc(fm.get('title'))}</strong></td>"
            f"<td>{esc(fm.get('indicator'))}</td><td>{esc(fm.get('threshold'))}</td>"
            f"<td>{esc(fm.get('response'))}</td><td>{esc(fm.get('mitigation'))}</td>"
            f"<td>{esc(fm.get('owner'))}</td></tr>")
    out.append("</table>")

    if data.get("residual_risk"):
        out.append("<h2>Residual risk (accepted knowingly)</h2>")
        out.append(f"<p>{esc(data['residual_risk'])}</p>")

    out.append("</body></html>")
    return "".join(out)


def parse_args(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--input", metavar="FILE", help="Analysis JSON file (default: stdin).")
    parser.add_argument("--output", metavar="FILE", help="Write HTML here instead of stdout.")
    parser.add_argument("--output-dir", metavar="DIR", help="Write RUN-ID-premortem-report.html into this directory.")
    parser.add_argument("--run-id", metavar="ID", help="Filename prefix for output-dir mode (default: today's date).")
    return parser.parse_args(argv)


def main(argv=None):
    args = parse_args(argv)
    if args.input:
        input_path = Path(args.input)
        if not input_path.exists():
            print(f"error: input file does not exist: {input_path}", file=sys.stderr)
            return 2
        raw = input_path.read_text(encoding="utf-8")
    else:
        raw = sys.stdin.read()
    if not raw.strip():
        print("error: no analysis JSON provided on --input or stdin", file=sys.stderr)
        return 2
    try:
        data = json.loads(raw)
    except json.JSONDecodeError as exc:
        print(f"error: invalid JSON in input: {exc}", file=sys.stderr)
        return 2
    report = render(data)
    if args.output_dir:
        out_dir = Path(args.output_dir)
        out_dir.mkdir(parents=True, exist_ok=True)
        run_id = args.run_id or date.today().isoformat()
        path = out_dir / f"{run_id}-premortem-report.html"
        path.write_text(report + "\n", encoding="utf-8")
        print(f"wrote {path}")
    elif args.output:
        Path(args.output).write_text(report + "\n", encoding="utf-8")
    else:
        print(report)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
