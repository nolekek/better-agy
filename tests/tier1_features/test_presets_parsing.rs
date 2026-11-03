use agy_core::{load_presets_from_str, ALWAYS_ACTIVE_META_SKILLS, REQUIRED_PRESETS};

const SAMPLE_VALID_PRESETS_YAML: &str = r#"
version: "3.0"

always_active:
  - "agent-preset-conductor"
  - "grill-me"
  - "systematic-debugging"
  - "verification-before-completion"
  - "concise-planning"

presets:
  web:
    name: "Web Development"
    skills:
      - "nextjs-fullstack-guru"
      - "react-best-practices"
      - "tailwind-responsive-design"
  backend:
    name: "Backend Engineering"
    skills:
      - "postgres-dba-optimizer"
      - "database-design"
      - "api-design-principles"
  devops:
    skills:
      - "docker-multi-stage"
      - "kubernetes-manifests"
  appsec:
    skills:
      - "owasp-top-10"
  qa:
    skills:
      - "playwright-e2e-testing"
  ai-agents:
    skills:
      - "mcp-server-developer"
  web-research:
    skills:
      - "deep-research-specialist"
  reverse-engineering:
    skills:
      - "binary-analysis-ninja"
  product:
    skills:
      - "monetization-strategy"
  orchestrator:
    skills:
      - "multi-agent-orchestrator"
  systems:
    skills:
      - "modern-cpp-systems"
  vst3:
    skills:
      - "juce-audio-dsp"
  hardware:
    skills:
      - "cadcraft-3d-modeling"
  media:
    skills:
      - "filmcraft-video-editing"
  meta-dev:
    skills:
      - "antigravity-core-dev"
  minimal:
    skills: []

hybrid_rules:
  - trigger: ["web", "qa"]
    compose: ["web", "qa"]
    description: "Web app with automated Playwright testing"
"#;

#[test]
fn test_f2_parse_valid_presets_manifest() {
    let manifest = load_presets_from_str(SAMPLE_VALID_PRESETS_YAML).expect("Parse valid manifest");
    assert_eq!(manifest.version, "3.0");
    assert_eq!(manifest.always_active.len(), 5);
    assert_eq!(manifest.presets.len(), 16);
}

#[test]
fn test_f2_verify_all_16_canonical_presets_exist() {
    let manifest = load_presets_from_str(SAMPLE_VALID_PRESETS_YAML).expect("Parse valid manifest");
    for &preset_name in &REQUIRED_PRESETS {
        assert!(
            manifest.presets.contains_key(preset_name),
            "Manifest must contain required preset: {}",
            preset_name
        );
    }
}

#[test]
fn test_f4_verify_all_5_always_active_meta_skills_exist() {
    let manifest = load_presets_from_str(SAMPLE_VALID_PRESETS_YAML).expect("Parse valid manifest");
    for &meta_skill in &ALWAYS_ACTIVE_META_SKILLS {
        assert!(
            manifest.always_active.iter().any(|s| s == meta_skill),
            "Manifest must contain mandatory meta-skill: {}",
            meta_skill
        );
    }
}

#[test]
fn test_f4_resolve_preset_always_injects_meta_skills() {
    let manifest = load_presets_from_str(SAMPLE_VALID_PRESETS_YAML).expect("Parse valid manifest");

    // Resolve 'backend' preset
    let active_skills = manifest
        .resolve_preset("backend")
        .expect("Resolve backend preset");

    // All 5 ALWAYS_ACTIVE skills must be present
    for &meta_skill in &ALWAYS_ACTIVE_META_SKILLS {
        assert!(
            active_skills.contains(&meta_skill.to_string()),
            "Resolved preset must include meta-skill {}",
            meta_skill
        );
    }

    // Backend domain skills must be present
    assert!(active_skills.contains(&"postgres-dba-optimizer".to_string()));
}

#[test]
fn test_f4_minimal_preset_contains_only_always_active_skills() {
    let manifest = load_presets_from_str(SAMPLE_VALID_PRESETS_YAML).expect("Parse valid manifest");

    let active_skills = manifest
        .resolve_preset("minimal")
        .expect("Resolve minimal preset");

    assert_eq!(
        active_skills.len(),
        5,
        "Minimal preset must contain exactly the 5 ALWAYS_ACTIVE skills"
    );

    for &meta_skill in &ALWAYS_ACTIVE_META_SKILLS {
        assert!(
            active_skills.contains(&meta_skill.to_string()),
            "Minimal preset must contain {}",
            meta_skill
        );
    }
}
