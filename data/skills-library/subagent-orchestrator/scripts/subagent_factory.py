#!/usr/bin/env python3
"""
Antigravity Subagent Factory & Preset Profile Compiler
Compiles specialized subagent definitions equipped with native Antigravity skills.
"""
import os
import json
import re

USER_HOME = os.path.expanduser("~")
GLOBAL_SKILLS_LIB = os.path.join(USER_HOME, ".gemini", "skills-library")
WORKSPACE_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
WORKSPACE_LIB = os.path.join(WORKSPACE_DIR, "skills-library")

PRESET_SKILLS = {
    "backend": {
        "name": "subagent-backend",
        "role": "Senior Backend & Database Engineer",
        "description": "Specialized Backend subagent equipped with PostgreSQL, database design, FastAPI, Node.js, Go, Redis, and API principles skills",
        "skills": [
            "database-design", "api-design-principles", "nodejs-best-practices",
            "go-concurrency-patterns", "memory-safety-patterns", "postgres-dba-optimizer",
            "fastapi-async-architect", "go-highload-services", "rust-systems-safe",
            "redis-distributed-caching", "kafka-event-streaming", "systematic-debugging",
            "verification-before-completion"
        ],
        "guidelines": [
            "Follow strict database normalization and migration safety.",
            "Design clean REST/GraphQL/gRPC APIs adhering to defined contracts.",
            "Write robust error handling and type-safe data access layers.",
            "Maintain Section 5 Workspace Hygiene: zero disposable scripts in workspace root."
        ]
    },
    "web": {
        "name": "subagent-web",
        "role": "Senior Frontend & Modern Web Engineer",
        "description": "Specialized Frontend subagent equipped with Next.js 15, React, Tailwind v4, Vue, UI/UX, SEO, and Web Vitals skills",
        "skills": [
            "ui-ux-pro-max", "frontend-design", "react-best-practices", "seo-audit",
            "form-cro", "nextjs-fullstack-guru", "tailwind-v4-master", "react-performance-opt",
            "vue-nuxt-expert", "web-state-query-master", "web-performance-auditor"
        ],
        "guidelines": [
            "Create modern, responsive, and accessible UI components.",
            "Follow React Best Practices: avoid unnecessary re-renders, hoist static elements.",
            "Adhere to Tailwind v4 and modern component system architecture.",
            "Maintain Section 5 Workspace Hygiene: zero disposable scripts in workspace root."
        ]
    },
    "qa": {
        "name": "subagent-qa",
        "role": "Senior QA Automation & Test Engineer",
        "description": "Specialized QA subagent equipped with Playwright E2E, Windows Desktop MCP control, TDD, test fixing, and k6 load testing skills",
        "skills": [
            "playwright-e2e-guru", "windows-desktop-control", "test-driven-development",
            "test-fixing", "code-review-checklist", "unit-test-synthesizer",
            "k6-load-testing-pro", "api-contract-tester"
        ],
        "guidelines": [
            "Write comprehensive Playwright E2E and unit tests covering edge cases.",
            "Verify 100% test pass before concluding any verification task.",
            "Conduct structured code reviews using code-review-checklist.",
            "Maintain Section 5 Workspace Hygiene: zero disposable scripts in workspace root."
        ]
    },
    "devops": {
        "name": "subagent-devops",
        "role": "Senior DevOps & Cloud Infrastructure Engineer",
        "description": "Specialized DevOps subagent equipped with Docker multi-stage, K8s, GitHub Actions CI/CD, Terraform, and Observability skills",
        "skills": [
            "docker-container-master", "k8s-manifest-generator", "github-actions-ci-cd",
            "terraform-iac-architect", "incident-responder", "deployment-procedures",
            "aws-serverless", "observability-telemetry-guru"
        ],
        "guidelines": [
            "Produce reproducible, secure multi-stage Dockerfiles and Helm/K8s manifests.",
            "Design zero-downtime CI/CD deployment pipelines.",
            "Follow Infrastructure as Code (IaC) best practices with Terraform.",
            "Maintain Section 5 Workspace Hygiene: zero disposable scripts in workspace root."
        ]
    },
    "appsec": {
        "name": "subagent-appsec",
        "role": "Senior Application Security & Reverse Engineer",
        "description": "Specialized AppSec subagent equipped with OWASP Top-10, API security, Electron ASAR reversing, Windows desktop automation, and pentest fuzzing skills",
        "skills": [
            "top-web-vulnerabilities", "api-security-best-practices", "ethical-hacking-methodology",
            "appsec-code-auditor", "reverse-engineer-electron", "win32-powershell-automation",
            "windows-desktop-control", "network-traffic-intercept", "api-pentest-fuzzer",
            "premortem-redteam"
        ],
        "guidelines": [
            "Audit code for OWASP Top-10, injection flaws, and auth bypasses.",
            "Inspect ASAR packages, binary unpacking, and native credential integrity.",
            "Red-team architectures and identify single points of failure.",
            "Maintain Section 5 Workspace Hygiene: zero disposable scripts in workspace root."
        ]
    },
    "product": {
        "name": "subagent-product",
        "role": "Product Partner & Monetization Strategist",
        "description": "Specialized Product subagent equipped with monetization strategies, pricing, unit economics, market intelligence, and CRO skills",
        "skills": [
            "product-monetization-strategy", "market-competitor-intelligence", "ai-product-partner",
            "browser", "search-specialist", "deep-research-synthesizer",
            "replica-entrepreneur", "replica-brand", "replica-launch", "form-cro", "ui-ux-pro-max", "seo-audit"
        ],
        "guidelines": [
            "Evaluate Marty Cagan's 4 Big Product Risks (Value, Usability, Feasibility, Viability).",
            "Scope MVPs ruthlessly using Shape Up appetite and MoSCoW prioritization.",
            "Design pricing tiers, Grand Slam Offers, and CRO conversion funnels.",
            "Maintain Section 5 Workspace Hygiene: zero disposable scripts in workspace root."
        ]
    },
    "web-research": {
        "name": "subagent-research",
        "role": "Deep Web Researcher & OSINT Analyst",
        "description": "Specialized Research subagent equipped with high-precision web search, deep synthesis, scraping, and OSINT skills",
        "skills": [
            "search-specialist", "deep-research-synthesizer", "web-scraper-harvester",
            "osint-recon-investigator"
        ],
        "guidelines": [
            "Use native search_web tool first; NEVER write ad-hoc scraping scripts in project root.",
            "Synthesize deep findings with citations and structured executive summaries.",
            "Maintain Section 5 Workspace Hygiene: zero disposable scripts in workspace root."
        ]
    },
    "reverse-engineering": {
        "name": "subagent-reverse",
        "role": "Clean-Room Reverse Engineering & Architecture Specialist",
        "description": "Specialized Reverse Engineering subagent equipped with clean-room replica cycle skills and Electron ASAR reversing",
        "skills": [
            "replica-recon", "replica-architect", "replica-backend", "replica-brand",
            "replica-build", "replica-deploy", "replica-design", "replica-diff",
            "replica-entrepreneur", "replica-launch", "replica-test",
            "reverse-engineer-electron"
        ],
        "guidelines": [
            "Follow strict clean-room reconstruction methodologies.",
            "Deconstruct protocols, schemas, and assets without contaminating licensing boundaries.",
            "Maintain Section 5 Workspace Hygiene: zero disposable scripts in workspace root."
        ]
    },
    "systems": {
        "name": "subagent-systems",
        "role": "Senior Systems & High-Performance C++ Engineer",
        "description": "Specialized Systems subagent equipped with Modern C++20/C++23, low-level optimization, lock-free concurrency, memory safety, SIMD, and ASan/TSan sanitizers",
        "skills": [
            "cpp-systems-architect", "systems-lowlevel-perf", "lockfree-concurrency-patterns",
            "rust-systems-safe", "memory-safety-patterns", "test-driven-development",
            "test-fixing", "code-review-checklist"
        ],
        "guidelines": [
            "Enforce strict C++ Core Guidelines, RAII, Rule of Zero/Five, and memory safety.",
            "Zero allocations and lock-free SPSC queues for latency-critical paths.",
            "Verify builds with ASan/TSan/UBSan sanitizers and run benchmarks.",
            "Maintain Section 5 Workspace Hygiene: zero disposable scripts in workspace root."
        ]
    },
    "vst3": {
        "name": "subagent-vst3",
        "role": "Senior Audio DSP & VST3 Plugin Engineer",
        "description": "Specialized Audio DSP subagent equipped with VST3/VST3i SDK, JUCE v7/v8, real-time audio thread safety, DSP algorithms, and audio UI/UX",
        "skills": [
            "vst3-dsp-architect", "audio-plugin-ui-ux", "cpp-systems-architect",
            "systems-lowlevel-perf", "lockfree-concurrency-patterns",
            "test-driven-development", "test-fixing", "code-review-checklist"
        ],
        "guidelines": [
            "Real-time audio thread safety: zero allocations (malloc/new), zero system calls, zero mutexes on audio thread.",
            "Master JUCE framework, modern LookAndFeel, and vector rotary sliders.",
            "Validate plugins with Steinberg pluginval at strictness level 10.",
            "Maintain Section 5 Workspace Hygiene: zero disposable scripts in workspace root."
        ]
    },
    "meta-dev": {
        "name": "subagent-meta-dev",
        "role": "Antigravity Core & Self-Evolution Engineer",
        "description": "Specialized internal developer subagent for Antigravity core maintenance, ASAR packing, multi-account carousel, and skill testing",
        "skills": [
            "antigravity-core-dev", "reverse-engineer-electron", "win32-powershell-automation",
            "windows-desktop-control", "test-driven-development", "test-fixing",
            "code-review-checklist", "nodejs-best-practices", "subagent-orchestrator",
            "hierarchical-task-decomposer", "github-actions-ci-cd"
        ],
        "guidelines": [
            "Strictly enforce 100% test pass (79/79) on all changes.",
            "Always synchronize changes between root, asar_extracted_228/dist/, and installation.",
            "Preserve ASAR unpack rules for chrome-devtools-mcp and credHelper.py.",
            "Maintain Section 5 Workspace Hygiene: zero disposable scripts in workspace root."
        ]
    }
}

def extract_description_from_file(content, skill_name):
    # Try frontmatter
    fm_match = re.search(r"^---\r?\n([\s\S]*?)\r?\n---", content)
    if fm_match:
        fm = fm_match.group(1)
        # Check multiline description: >- or |
        m_multi = re.search(r"^description:\s*[>|]-?\r?\n((?:[ \t]+.+\r?\n?)+)", fm, re.MULTILINE)
        if m_multi:
            lines = [l.strip() for l in m_multi.group(1).splitlines() if l.strip()]
            return " ".join(lines)
        m_single = re.search(r"^description:\s*(.+)$", fm, re.MULTILINE)
        if m_single:
            desc = m_single.group(1).strip().strip('"\'')
            if desc and desc not in [">-", "|-", ">", "|"]:
                return desc
    # Fallback to H1
    h1_match = re.search(r"^#\s+(.+)$", content, re.MULTILINE)
    if h1_match:
        return h1_match.group(1).strip()
    return f"Specialized instructions for {skill_name}"

def get_skill_metadata(skill_name):
    # Check library folders where SKILL.md is never disabled
    search_dirs = [
        os.path.join(GLOBAL_SKILLS_LIB, skill_name),
        os.path.join(WORKSPACE_LIB, skill_name),
        os.path.join(USER_HOME, ".gemini", "config", "skills", skill_name)
    ]
    for sp in search_dirs:
        for fname in ["SKILL.md", "skill.md", "SKILL.md.disabled", "skill.md.disabled"]:
            fpath = os.path.join(sp, fname)
            if os.path.exists(fpath):
                try:
                    with open(fpath, "r", encoding="utf-8") as f:
                        content = f.read(4096)
                    desc = extract_description_from_file(content, skill_name)
                    # Always provide the stable library path with SKILL.md
                    canonical_path = os.path.join(GLOBAL_SKILLS_LIB, skill_name, "SKILL.md").replace("\\", "/")
                    return {
                        "name": skill_name,
                        "description": desc,
                        "path": canonical_path
                    }
                except Exception:
                    pass
    canonical_path = f"{GLOBAL_SKILLS_LIB}/{skill_name}/SKILL.md".replace("\\", "/")
    return {
        "name": skill_name,
        "description": f"Specialized instructions for {skill_name}",
        "path": canonical_path
    }

def compile_subagent_profile(preset_id):
    if preset_id not in PRESET_SKILLS:
        raise ValueError(f"Unknown preset: {preset_id}. Available: {list(PRESET_SKILLS.keys())}")
    
    cfg = PRESET_SKILLS[preset_id]
    skills_meta = [get_skill_metadata(s) for s in cfg["skills"]]
    
    skills_block = "<skills>\nYou can use specialized 'skills' to help you with complex tasks. Each skill has instructions, scripts, and resources that extend your capabilities.\nIf a skill is relevant to your current task, you MUST use read_file to read its SKILL.md before proceeding.\n\nAvailable skills:\n"
    for sm in skills_meta:
        skills_block += f"- {sm['name']} ({sm['path']}): {sm['description']}\n"
    skills_block += "</skills>\n"
    
    guidelines_text = "\n".join(f"- {g}" for g in cfg["guidelines"])
    
    system_prompt = f"""You are a specialized autonomous AI subagent operating as: {cfg['role']}.
You were spawned by the Antigravity Orchestrator to execute a focused technical mission.

## Core Mandates & Guidelines
{guidelines_text}

## Available Specialized Skills
{skills_block}

## Instructions for Execution
1. Analyze your assigned prompt and check the relevant skills in <skills> using `read_file`.
2. Execute the task cleanly and incrementally.
3. Before reporting completion, ALWAYS verify your work (run tests or validate syntax).
4. Do NOT pollute the workspace: never create disposable research or scrape scripts in the root directory.
5. Once your task is finished, return a structured executive summary to the orchestrator with:
   - Status (✔ Complete / ⚠ Blocked)
   - Files modified or created
   - Verification evidence (test pass count or syntax check)
   - Key decisions and findings
"""

    return {
        "Name": cfg["name"],
        "Description": cfg["description"],
        "Role": cfg["role"],
        "EnableWriteTools": True,
        "EnableMCPTools": True,
        "EnableSubagentTools": False,
        "SystemPrompt": system_prompt
    }

def build_all_manifest():
    manifest = {}
    for p_id in PRESET_SKILLS:
        manifest[p_id] = compile_subagent_profile(p_id)
    return manifest

if __name__ == "__main__":
    import sys
    manifest = build_all_manifest()
    manifest_path = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "subagents_manifest.json")
    with open(manifest_path, "w", encoding="utf-8") as f:
        json.dump(manifest, f, indent=2, ensure_ascii=False)
    print(f"✔ Subagents manifest generated with {len(manifest)} presets at: {manifest_path}")
    
    # Also sync manifest to ~/.gemini/skills-library/subagent-orchestrator and config/skills/subagent-orchestrator
    dst_global = os.path.join(USER_HOME, ".gemini", "skills-library", "subagent-orchestrator", "subagents_manifest.json")
    dst_config = os.path.join(USER_HOME, ".gemini", "config", "skills", "subagent-orchestrator", "subagents_manifest.json")
    try:
        os.makedirs(os.path.dirname(dst_global), exist_ok=True)
        with open(dst_global, "w", encoding="utf-8") as f:
            json.dump(manifest, f, indent=2, ensure_ascii=False)
        os.makedirs(os.path.dirname(dst_config), exist_ok=True)
        with open(dst_config, "w", encoding="utf-8") as f:
            json.dump(manifest, f, indent=2, ensure_ascii=False)
        print("✔ Subagents manifest synced to ~/.gemini/skills-library and config/skills.")
    except Exception as e:
        print(f"Sync warning: {e}")
