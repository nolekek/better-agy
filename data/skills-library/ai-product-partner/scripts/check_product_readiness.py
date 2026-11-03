#!/usr/bin/env python3
"""
Product Readiness Validator (check_product_readiness.py)
Part of the AI-Product-Partner Skill.

Analyzes PRD / Product Spec markdown documents against product excellence standards:
- Problem & JTBD Definition
- Marty Cagan's 4 Big Risks (Value, Usability, Feasibility, Viability)
- User Flows & 5-State UX Matrix (Empty, Loading, Error, Edge cases)
- Scope Boundaries & Shape Up Appetite (In-Scope vs Out-of-Scope)
- Telemetry & Success Metrics (North Star, Event Schema)
- Anti-Pattern Detection (Vague buzzwords, open-ended scope, missing recovery states)
"""

import argparse
import json
import os
import re
import sys
from typing import Dict, List, Tuple


class ProductReadinessAuditor:
    def __init__(self, content: str, file_path: str = ""):
        self.content = content
        self.file_path = file_path
        self.lower_content = content.lower()
        self.findings: List[Dict[str, str]] = []
        self.scores: Dict[str, Tuple[float, float]] = {}  # category: (earned, max)
        self.anti_patterns: List[Dict[str, str]] = []

    def audit(self) -> Dict:
        # 1. Problem & JTBD (15 pts)
        self._check_problem_and_jtbd()

        # 2. Marty Cagan's 4 Risks (20 pts)
        self._check_cagan_risks()

        # 3. User Flows & State Architecture (25 pts)
        self._check_user_flows_and_states()

        # 4. Scope Boundaries & Appetite (20 pts)
        self._check_scope_and_appetite()

        # 5. Metrics & Telemetry (15 pts)
        self._check_metrics_and_telemetry()

        # 6. Anti-Patterns Check (deductions)
        self._check_anti_patterns()

        total_earned = sum(earned for earned, _ in self.scores.values())
        total_max = sum(max_pts for _, max_pts in self.scores.values())
        raw_percentage = (total_earned / total_max) * 100 if total_max > 0 else 0

        # Apply penalty deductions for critical anti-patterns (up to -20 pts)
        deduction = min(len(self.anti_patterns) * 4, 20)
        final_score = max(0.0, round(raw_percentage - deduction, 1))

        status = "PASSED" if final_score >= 75.0 else "NEEDS_IMPROVEMENT"
        if final_score < 50.0:
            status = "CRITICAL_GAPS"

        return {
            "file": self.file_path,
            "status": status,
            "readiness_score": final_score,
            "raw_score": round(raw_percentage, 1),
            "penalty_deductions": deduction,
            "category_breakdown": {
                k: {"score": earned, "max": max_pts, "percentage": round((earned / max_pts) * 100, 1)}
                for k, (earned, max_pts) in self.scores.items()
            },
            "findings": self.findings,
            "anti_patterns": self.anti_patterns,
        }

    def _check_problem_and_jtbd(self):
        max_pts = 15.0
        earned = 0.0

        # Problem statement check
        has_problem = any(kw in self.lower_content for kw in ["problem statement", "проблема", "problem"])
        if has_problem:
            earned += 5.0
            self.findings.append({"level": "OK", "msg": "Problem statement is defined."})
        else:
            self.findings.append({"level": "WARN", "msg": "Missing clear Problem Statement."})

        # JTBD check (Jobs to be Done)
        has_jtbd = bool(
            re.search(r"when\s+.*,\s*i\s+want\s+to\s+.*,\s*so\s+i\s+can", self.lower_content)
            or re.search(r"когда\s+.*,\s*я\s+хочу\s+.*,\s*чтобы", self.lower_content)
            or "jobs to be done" in self.lower_content
            or "jtbd" in self.lower_content
        )
        if has_jtbd:
            earned += 5.0
            self.findings.append({"level": "OK", "msg": "Jobs to be Done (JTBD) structure is present."})
        else:
            self.findings.append({"level": "WARN", "msg": "No standard Jobs to be Done (JTBD) framing found."})

        # Personas and Anti-Persona
        has_persona = "persona" in self.lower_content or "целевая аудитория" in self.lower_content
        has_anti_persona = "anti-persona" in self.lower_content or "не для" in self.lower_content or "out of scope persona" in self.lower_content
        if has_persona:
            earned += 3.0
        if has_anti_persona:
            earned += 2.0
            self.findings.append({"level": "OK", "msg": "Target Persona and Anti-Persona defined."})
        else:
            self.findings.append({"level": "INFO", "msg": "Consider defining an Anti-Persona to prevent feature creep."})

        self.scores["Problem & Value (JTBD)"] = (earned, max_pts)

    def _check_cagan_risks(self):
        max_pts = 20.0
        earned = 0.0
        risks_found = []

        # Value Risk
        if any(kw in self.lower_content for kw in ["value risk", "риск ценности", "ценность"]):
            earned += 5.0
            risks_found.append("Value")

        # Usability Risk
        if any(kw in self.lower_content for kw in ["usability risk", "риск юзабилити", "юзабилити", "convenience"]):
            earned += 5.0
            risks_found.append("Usability")

        # Feasibility Risk
        if any(kw in self.lower_content for kw in ["feasibility risk", "риск реализуемости", "технический риск", "feasibility"]):
            earned += 5.0
            risks_found.append("Feasibility")

        # Viability Risk
        if any(kw in self.lower_content for kw in ["viability risk", "риск жизнеспособности", "бизнес-риск", "viability", "economics"]):
            earned += 5.0
            risks_found.append("Viability")

        if len(risks_found) >= 3:
            self.findings.append({"level": "OK", "msg": f"Evaluated Cagan Risks ({', '.join(risks_found)})."})
        else:
            self.findings.append({"level": "WARN", "msg": f"Incomplete Cagan Risk evaluation. Found: {risks_found}. Expected: Value, Usability, Feasibility, Viability."})

        self.scores["Cagan 4 Big Risks"] = (earned, max_pts)

    def _check_user_flows_and_states(self):
        max_pts = 25.0
        earned = 0.0

        # User flow / Journey
        has_flow = any(kw in self.lower_content for kw in ["user journey", "user flow", "happy path", "сценарий", "путь пользователя"])
        if has_flow:
            earned += 7.0
            self.findings.append({"level": "OK", "msg": "User Flow / Happy path documented."})
        else:
            self.findings.append({"level": "WARN", "msg": "Missing User Flow / Journey steps."})

        # 5-State Matrix
        states_found = []
        if any(kw in self.lower_content for kw in ["empty state", "zero data", "пустое состояние"]):
            earned += 5.0
            states_found.append("Empty")
        if any(kw in self.lower_content for kw in ["loading state", "skeleton", "spinner", "загрузка", "progress"]):
            earned += 4.0
            states_found.append("Loading")
        if any(kw in self.lower_content for kw in ["error state", "recovery", "ошибка", "error message"]):
            earned += 5.0
            states_found.append("Error")
        if any(kw in self.lower_content for kw in ["edge case", "boundary", "крайний случай", "таймаут"]):
            earned += 4.0
            states_found.append("Edge Cases")

        if len(states_found) >= 3:
            self.findings.append({"level": "OK", "msg": f"UX States designed: ({', '.join(states_found)})."})
        else:
            self.findings.append({"level": "WARN", "msg": f"Missing critical UX states. Found: {states_found}. Ensure Empty, Loading, Error, and Edge cases are explicitly designed."})

        self.scores["User Flow & State Architecture"] = (earned, max_pts)

    def _check_scope_and_appetite(self):
        max_pts = 20.0
        earned = 0.0

        # In-Scope items
        has_in_scope = any(kw in self.lower_content for kw in ["in-scope", "must have", "must-have", "в скоупе", "обязательно"])
        if has_in_scope:
            earned += 7.0
            self.findings.append({"level": "OK", "msg": "In-Scope (Must-Haves) defined."})
        else:
            self.findings.append({"level": "WARN", "msg": "Missing explicit In-Scope boundaries."})

        # Out-of-Scope items (CRITICAL)
        has_out_of_scope = any(kw in self.lower_content for kw in ["out-of-scope", "out of scope", "won't have", "вне скоупа", "не входит в mvp"])
        if has_out_of_scope:
            earned += 8.0
            self.findings.append({"level": "OK", "msg": "Out-of-Scope boundary explicitly declared (anti-bloat shield)."})
        else:
            self.findings.append({"level": "WARN", "msg": "Missing Out-of-Scope declaration. High risk of scope creep!"})

        # Timebox / Appetite (Shape Up)
        has_appetite = any(kw in self.lower_content for kw in ["appetite", "timebox", "спринт", "таймбокс", "длительность"])
        if has_appetite:
            earned += 5.0
            self.findings.append({"level": "OK", "msg": "Appetite / Timebox defined (Shape Up)."})
        else:
            self.findings.append({"level": "INFO", "msg": "Set a fixed appetite (e.g. 1 week) to constrain scope."})

        self.scores["Scope & Appetite Boundaries"] = (earned, max_pts)

    def _check_metrics_and_telemetry(self):
        max_pts = 15.0
        earned = 0.0

        # North Star Metric / Core Metric
        has_nsm = any(kw in self.lower_content for kw in ["north star", "главная метрика", "success metric", "метрика успеха"])
        if has_nsm:
            earned += 8.0
            self.findings.append({"level": "OK", "msg": "North Star / Primary Success Metric specified."})
        else:
            self.findings.append({"level": "WARN", "msg": "Missing concrete North Star / Primary Success Metric."})

        # Telemetry / Event tracking schema
        has_telemetry = any(kw in self.lower_content for kw in ["telemetry", "event tracking", "телеметрия", "tracking schema", "events"])
        if has_telemetry:
            earned += 7.0
            self.findings.append({"level": "OK", "msg": "Telemetry event schema present."})
        else:
            self.findings.append({"level": "WARN", "msg": "No telemetry or event tracking defined."})

        self.scores["Metrics & Telemetry"] = (earned, max_pts)

    def _check_anti_patterns(self):
        # 1. Vague buzzwords without measurable definition
        vague_terms = [
            (r"\b(intuitive|user-friendly|easy to use|простой и понятный)\b", "Vague UX phrasing ('intuitive' / 'user-friendly'). Define concrete behavioral metrics or task completion time instead."),
            (r"\b(blazing fast|super fast|очень быстрый)\b", "Subjective performance claim ('blazing fast'). Specify numeric SLA (e.g. P95 latency < 200ms)."),
            (r"\b(users will love it|пользователям понравится)\b", "Speculative value claim without user validation or JTBD grounding."),
        ]
        for pattern, explanation in vague_terms:
            if re.search(pattern, self.lower_content):
                self.anti_patterns.append({
                    "pattern": "Vague Quality Claims",
                    "detail": explanation
                })

        # 2. Missing Out-of-Scope while having extensive feature lists
        if "out-of-scope" not in self.lower_content and "вне скоупа" not in self.lower_content:
            if self.lower_content.count("- [ ]") > 5 or self.lower_content.count("- [x]") > 5:
                self.anti_patterns.append({
                    "pattern": "Unbounded Feature List",
                    "detail": "Over 5 feature check-items without an explicit 'Out-of-Scope' section to prevent creep."
                })

        # 3. Unresolved TODOs or placeholders in approved PRD
        todos = re.findall(r"(TODO|TBD|FIXME|\[\.\.\.\])", self.content)
        if len(todos) > 3:
            self.anti_patterns.append({
                "pattern": "Excessive Placeholders",
                "detail": f"Document contains {len(todos)} unresolved placeholders (TODO/TBD/FIXME)."
            })


def format_cli_output(report: Dict) -> str:
    lines = []
    lines.append("=" * 64)
    lines.append("        AI-PRODUCT-PARTNER: PRODUCT READINESS AUDIT")
    lines.append("=" * 64)
    lines.append(f"Target Document: {report['file'] or 'Standard Input'}")
    
    status_icon = "✓" if report['status'] == "PASSED" else "⚠" if report['status'] == "NEEDS_IMPROVEMENT" else "✗"
    lines.append(f"Readiness Score: {report['readiness_score']}%  [{status_icon} {report['status']}]")
    if report['penalty_deductions'] > 0:
        lines.append(f"Deductions: -{report['penalty_deductions']} pts (due to anti-patterns detected)")
    lines.append("-" * 64)

    lines.append("\n[CATEGORY BREAKDOWN]")
    for cat, data in report["category_breakdown"].items():
        bar_len = int(data["percentage"] / 10)
        bar = "■" * bar_len + "□" * (10 - bar_len)
        lines.append(f"  {cat:<32} {bar} {data['score']:>4.1f}/{data['max']:<4.1f} ({data['percentage']:>5.1f}%)")

    lines.append("\n[KEY FINDINGS]")
    for finding in report["findings"]:
        prefix = "[+]" if finding["level"] == "OK" else "[!]" if finding["level"] == "WARN" else "[i]"
        lines.append(f"  {prefix} {finding['msg']}")

    if report["anti_patterns"]:
        lines.append("\n[ANTI-PATTERNS DETECTED]")
        for ap in report["anti_patterns"]:
            lines.append(f"  [X] {ap['pattern']}: {ap['detail']}")

    lines.append("\n" + "=" * 64)
    if report['status'] == "PASSED":
        lines.append("RESULT: READY FOR TECHNICAL IMPLEMENTATION. High product clarity.")
    elif report['status'] == "NEEDS_IMPROVEMENT":
        lines.append("RESULT: REVIEW RECOMMENDED. Address the [!] warnings before coding.")
    else:
        lines.append("RESULT: CRITICAL PRODUCT GAPS. High risk of building the wrong thing.")
    lines.append("=" * 64)
    return "\n".join(lines)


def format_markdown_report(report: Dict) -> str:
    md = []
    md.append(f"# Product Readiness Audit Report: {os.path.basename(report['file']) if report['file'] else 'Document'}\n")
    md.append(f"**Overall Readiness Score**: `{report['readiness_score']}%` | **Status**: `{report['status']}`\n")
    md.append("## Category Breakdown\n")
    md.append("| Category | Score | Max | Completion |")
    md.append("| :--- | :--- | :--- | :--- |")
    for cat, data in report["category_breakdown"].items():
        md.append(f"| **{cat}** | {data['score']} | {data['max']} | {data['percentage']}% |")
    
    md.append("\n## Audit Findings\n")
    for f in report["findings"]:
        icon = "✅" if f["level"] == "OK" else "⚠️" if f["level"] == "WARN" else "ℹ️"
        md.append(f"- {icon} **[{f['level']}]** {f['msg']}")

    if report["anti_patterns"]:
        md.append("\n## Anti-Patterns & Product Risks\n")
        for ap in report["anti_patterns"]:
            md.append(f"- 🛑 **{ap['pattern']}**: {ap['detail']}")

    return "\n".join(md)


def main():
    parser = argparse.ArgumentParser(description="Audit PRD or Product Spec for product readiness.")
    parser.add_argument("--file", "-f", help="Path to markdown PRD file to inspect.", required=True)
    parser.add_argument("--json", action="store_true", help="Output results in machine-readable JSON format.")
    parser.add_argument("--report", "-r", help="Path to write a markdown report file.")
    args = parser.parse_args()

    if not os.path.isfile(args.file):
        print(f"Error: File not found: {args.file}", file=sys.stderr)
        sys.exit(2)

    with open(args.file, "r", encoding="utf-8") as f:
        content = f.read()

    auditor = ProductReadinessAuditor(content, file_path=args.file)
    report = auditor.audit()

    if args.json:
        print(json.dumps(report, indent=2, ensure_ascii=False))
    else:
        print(format_cli_output(report))

    if args.report:
        report_md = format_markdown_report(report)
        with open(args.report, "w", encoding="utf-8") as rf:
            rf.write(report_md)
        print(f"\nSaved markdown report to: {args.report}")

    sys.exit(0 if report["status"] == "PASSED" else 1)


if __name__ == "__main__":
    main()
