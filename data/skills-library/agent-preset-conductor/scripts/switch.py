#!/usr/bin/env python3
"""
Agent Preset Conductor - High-Speed Multi-Tier Preset Switcher
Tier 0: Direct Compiled Rust CLI `agy` (<5ms execution via daemon IPC or standalone)
Tier 1: Local HTTP IPC Bridge (http://127.0.0.1:51122/api/switch-preset)
Tier 2: Direct atomic filesystem toggle fallback (~/.gemini/config/skills/)
"""
import sys
import json
import os
import shutil
import subprocess
import urllib.request
import urllib.error
import time

BRIDGE_SWITCH_URL = "http://127.0.0.1:51122/api/switch-preset"
CONFIG_FILE = os.path.expanduser(r"~\.gemini\antigravity\control-center-config.json")
GLOBAL_SKILLS_DIR = os.path.expanduser(r"~\.gemini\config\skills")

ALWAYS_ACTIVE = {
    "agent-preset-conductor",
    "grill-me",
    "systematic-debugging",
    "verification-before-completion",
    "concise-planning"
}

PRESET_SKILLS_MAP = {
    "web": [
        "ui-ux-pro-max", "frontend-design", "react-best-practices", "seo-audit", "form-cro",
        "nextjs-fullstack-guru", "tailwind-v4-master", "react-performance-opt", 
        "vue-nuxt-expert", "web-state-query-master", "web-performance-auditor", "figcraft-design", "vectorcraft-artwork", 
        "agent-preset-conductor", "ai-product-partner", "modern-web-guidance", "generative_ui"
    ],
    "backend": [
        "database-design", "api-design-principles", "nodejs-best-practices",
        "go-concurrency-patterns", "memory-safety-patterns",
        "postgres-dba-optimizer", "fastapi-async-architect", "go-highload-services", 
        "rust-systems-safe", "redis-distributed-caching", "kafka-event-streaming", 
        "agent-preset-conductor", "ai-product-partner"
    ],
    "devops": [
        "incident-responder", "deployment-procedures", "aws-serverless",
        "docker-container-master", "k8s-manifest-generator", "github-actions-ci-cd", 
        "terraform-iac-architect", "observability-telemetry-guru", 
        "agent-preset-conductor", "ai-product-partner"
    ],
    "appsec": [
        "top-web-vulnerabilities", "api-security-best-practices", "ethical-hacking-methodology",
        "appsec-code-auditor", "craft-anything", "reverse-engineer-electron", "win32-powershell-automation", "windows-desktop-control", 
        "network-traffic-intercept", "api-pentest-fuzzer", 
        "agent-preset-conductor", "premortem-redteam"
    ],
    "qa": [
        "test-driven-development", "test-fixing", "code-review-checklist",
        "playwright-e2e-guru", "windows-desktop-control", "unit-test-synthesizer", "k6-load-testing-pro", 
        "api-contract-tester", "agent-preset-conductor", "ai-product-partner"
    ],
    "ai-agents": [
        "mcp-builder", "prompt-engineering", "rag-implementation",
        "context-window-management", "ai-agents-architect", "agent-evaluation",
        "langgraph", "browser", "fl-freelance-agent", "agent-preset-conductor", "ai-product-partner"
    ],
    "web-research": [
        "search-specialist", "deep-research-synthesizer", "web-scraper-harvester", 
        "osint-recon-investigator", "agent-preset-conductor", "ai-product-partner"
    ],
    "reverse-engineering": [
        "craft-anything", "replica-recon", "replica-architect", "replica-backend", "replica-brand", 
        "replica-build", "replica-deploy", "replica-design", "replica-diff", 
        "replica-entrepreneur", "replica-launch", "replica-test", 
        "reverse-engineer-electron", "agent-preset-conductor", "ai-product-partner"
    ],
    "product": [
        "product-monetization-strategy", "market-competitor-intelligence", "ai-product-partner",
        "browser", "search-specialist", "deep-research-synthesizer",
        "replica-entrepreneur", "replica-brand", "replica-launch",
        "form-cro", "ui-ux-pro-max", "seo-audit", "agent-preset-conductor"
    ],
    "orchestrator": [
        "subagent-orchestrator", "hierarchical-task-decomposer", "multi-agent-debate-consensus",
        "context-synthesis-reducer", "ai-agents-architect", "prompt-engineering",
        "context-window-management", "agent-evaluation", "langgraph",
        "artcraft-orchestrator", "craft-anything", "figcraft-design",
        "code-review-checklist", "premortem-redteam", "ai-product-partner", "agent-preset-conductor"
    ],
    "systems": [
        "cpp-systems-architect", "systems-lowlevel-perf", "lockfree-concurrency-patterns",
        "rust-systems-safe", "memory-safety-patterns", "test-driven-development",
        "test-fixing", "code-review-checklist", "agent-preset-conductor"
    ],
    "vst3": [
        "vst3-dsp-architect", "audio-plugin-ui-ux", "figcraft-design", "vectorcraft-artwork", "cpp-systems-architect",
        "systems-lowlevel-perf", "lockfree-concurrency-patterns",
        "subagent-orchestrator", "hierarchical-task-decomposer",
        "test-driven-development", "test-fixing", "code-review-checklist",
        "agent-preset-conductor"
    ],
    "hardware": [
        "solidcraft-modeling", "cadcraft-drafting", "texturecraft-pbr",
        "cpp-systems-architect", "systems-lowlevel-perf",
        "agent-preset-conductor", "ai-product-partner"
    ],
    "media": [
        "filmcraft-video", "effectcraft-vfx", "lightcraft-photo",
        "photocraft-studio", "vectorcraft-artwork", "figcraft-design",
        "ui-ux-pro-max", "frontend-design", "ai-product-partner",
        "agent-preset-conductor"
    ],
    "meta-dev": [
        "antigravity-core-dev", "reverse-engineer-electron", "win32-powershell-automation",
        "windows-desktop-control", "test-driven-development", "test-fixing",
        "code-review-checklist", "nodejs-best-practices", "subagent-orchestrator",
        "hierarchical-task-decomposer", "github-actions-ci-cd", "agent-preset-conductor"
    ],
    "minimal": [
        "agent-preset-conductor", "ai-product-partner"
    ]
}

def find_agy_binary():
    # 1. Check in PATH
    bin_path = shutil.which("agy.exe") or shutil.which("agy")
    if bin_path and os.path.isfile(bin_path):
        return bin_path

    # 2. Check in ~/.gemini/antigravity/bin/
    home_bin = os.path.expanduser(r"~\.gemini\antigravity\bin\agy.exe")
    if os.path.isfile(home_bin):
        return home_bin
    home_bin_nix = os.path.expanduser(r"~/.gemini/antigravity/bin/agy")
    if os.path.isfile(home_bin_nix):
        return home_bin_nix

    # 3. Check workspace and build targets
    script_dir = os.path.dirname(os.path.abspath(__file__))
    candidates = [
        os.path.join(script_dir, "..", "agy", "target", "release", "agy.exe"),
        os.path.join(script_dir, "..", "..", "agy", "target", "release", "agy.exe"),
        os.path.join(script_dir, "..", "..", "..", "agy", "target", "release", "agy.exe"),
        r"C:\Users\Admin\antigravityBUTmultiacc\agy\target\release\agy.exe",
        os.path.join(os.getcwd(), "agy", "target", "release", "agy.exe"),
        os.path.join(script_dir, "..", "agy", "target", "release", "agy"),
        os.path.join(script_dir, "..", "..", "agy", "target", "release", "agy")
    ]
    for c in candidates:
        norm = os.path.normpath(c)
        if os.path.isfile(norm):
            return norm

    return None

def update_config_active_preset(preset_name):
    try:
        cfg = {}
        if os.path.exists(CONFIG_FILE):
            with open(CONFIG_FILE, "r", encoding="utf-8") as f:
                cfg = json.load(f)
        cfg["activePreset"] = preset_name
        os.makedirs(os.path.dirname(CONFIG_FILE), exist_ok=True)
        with open(CONFIG_FILE, "w", encoding="utf-8") as f:
            json.dump(cfg, f, indent=2, ensure_ascii=False)
    except Exception:
        pass

def toggle_skill_dir(dir_path, should_be_active):
    active_file = os.path.join(dir_path, "SKILL.md")
    disabled_file = os.path.join(dir_path, "SKILL.md.disabled")
    lower_active = os.path.join(dir_path, "skill.md")
    lower_disabled = os.path.join(dir_path, "skill.md.disabled")

    for _ in range(5):
        try:
            if should_be_active:
                if os.path.exists(disabled_file):
                    os.replace(disabled_file, active_file)
                elif os.path.exists(lower_disabled):
                    os.replace(lower_disabled, lower_active)
            else:
                if os.path.exists(active_file):
                    os.replace(active_file, disabled_file)
                elif os.path.exists(lower_active):
                    os.replace(lower_active, lower_disabled)
            return True
        except (OSError, PermissionError):
            time.sleep(0.04)
    return False

def direct_fs_switch(preset_name):
    if not os.path.exists(GLOBAL_SKILLS_DIR):
        return False

    target_skills = set(PRESET_SKILLS_MAP.get(preset_name, []))

    active_count = 0
    total_count = 0
    activated_skills = []

    try:
        for entry in os.listdir(GLOBAL_SKILLS_DIR):
            p = os.path.join(GLOBAL_SKILLS_DIR, entry)
            if os.path.isdir(p):
                total_count += 1
                should_be_active = entry in ALWAYS_ACTIVE or entry in target_skills
                toggle_skill_dir(p, should_be_active)
                if should_be_active:
                    active_count += 1
                    activated_skills.append(entry)

        update_config_active_preset(preset_name)
        print(f"✔ Successfully switched to preset: '{preset_name}' (direct filesystem mode)")
        print(f"Active skills ({active_count}/{total_count}): {', '.join(sorted(activated_skills))}")
        return True
    except Exception as e:
        print(f"❌ Direct filesystem switch error: {e}")
        return False

def switch_preset(preset_name):
    # Tier 1: Try HTTP IPC bridge
    payload = json.dumps({"preset": preset_name}).encode("utf-8")
    req = urllib.request.Request(BRIDGE_SWITCH_URL, data=payload, headers={"Content-Type": "application/json"}, method="POST")
    try:
        with urllib.request.urlopen(req, timeout=1.5) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            if data.get("success"):
                update_config_active_preset(preset_name)
                print(f"✔ Successfully switched to preset: '{data.get('preset')}'")
                print(f"Active skills ({len(data.get('activeSkills', []))}): {', '.join(data.get('activeSkills', []))}")
                print(f"Estimated context: ~{data.get('totalActiveTokens', 0)} tokens")
                return True
    except Exception:
        # Bridge unreachable -> seamlessly fall back to Tier 2
        pass

    # Tier 2: Direct filesystem fallback
    return direct_fs_switch(preset_name)

def main():
    valid_presets = list(PRESET_SKILLS_MAP.keys())
    if len(sys.argv) < 2:
        print(f"Usage: python switch.py <preset_name> [overlays...] ({'|'.join(valid_presets)})")
        sys.exit(1)

    # Tier 0: Direct delegation to compiled high-speed Rust CLI `agy` (<5ms response)
    agy_bin = find_agy_binary()
    if agy_bin:
        try:
            cmd = [agy_bin, "preset", "switch"] + sys.argv[1:]
            res = subprocess.run(cmd)
            sys.exit(res.returncode)
        except Exception:
            # If agy fails unexpectedly, continue to fallback tiers
            pass

    # Fallback path if agy binary is absent
    target_preset = sys.argv[1].strip().lower()
    if target_preset not in PRESET_SKILLS_MAP:
        print(f"❌ Unknown preset: '{target_preset}'. Available ({len(PRESET_SKILLS_MAP)} presets): {', '.join(valid_presets)}")
        sys.exit(1)

    success = switch_preset(target_preset)
    sys.exit(0 if success else 1)

if __name__ == "__main__":
    main()
