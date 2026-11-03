use std::path::PathBuf;
use agy_core::{
    load_presets, load_presets_from_str, CoreError,
    ALWAYS_ACTIVE_META_SKILLS, REQUIRED_PRESETS,
};

fn library_presets_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("data")
        .join("skills-library")
        .join("presets.yaml")
}

fn library_skills_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("data")
        .join("skills-library")
}

#[test]
fn test_load_real_presets_yaml() {
    let path = library_presets_path();
    assert!(path.exists(), "presets.yaml must exist at {}", path.display());

    let manifest = load_presets(&path).expect("Failed to load and validate presets.yaml");

    // 1. Validate version
    assert_eq!(manifest.version, "3.0");

    // 2. Validate exactly 16 domain presets
    assert_eq!(manifest.presets.len(), 16);
    for &required_preset in &REQUIRED_PRESETS {
        assert!(
            manifest.presets.contains_key(required_preset),
            "Manifest missing required preset '{}'",
            required_preset
        );
    }

    // 3. Validate 5 ALWAYS_ACTIVE meta-skills in manifest declaration
    assert_eq!(manifest.always_active.len(), 5);
    for &meta_skill in &ALWAYS_ACTIVE_META_SKILLS {
        assert!(
            manifest.always_active.contains(&meta_skill.to_string()),
            "always_active missing '{}'",
            meta_skill
        );
    }
}

#[test]
fn test_all_16_presets_include_always_active_meta_skills() {
    let manifest = load_presets(library_presets_path()).unwrap();

    for &preset_name in &REQUIRED_PRESETS {
        let skills = manifest
            .resolve_preset(preset_name)
            .unwrap_or_else(|e| panic!("Failed to resolve preset '{}': {:?}", preset_name, e));

        // Every preset MUST include all 5 ALWAYS_ACTIVE meta-skills
        for &meta_skill in &ALWAYS_ACTIVE_META_SKILLS {
            assert!(
                skills.contains(&meta_skill.to_string()),
                "Preset '{}' is missing mandatory meta-skill '{}'",
                preset_name,
                meta_skill
            );
        }
    }
}

#[test]
fn test_minimal_preset_resolution() {
    let manifest = load_presets(library_presets_path()).unwrap();

    let skills = manifest.resolve_preset("minimal").unwrap();
    // In presets.yaml, minimal has domain skill [ai-product-partner] + 5 ALWAYS_ACTIVE meta-skills = 6 skills
    assert_eq!(
        skills.len(),
        6,
        "Minimal preset must contain 6 skills (5 ALWAYS_ACTIVE + ai-product-partner), got: {:?}",
        skills
    );

    // Verify all 5 ALWAYS_ACTIVE meta-skills are present
    for &meta_skill in &ALWAYS_ACTIVE_META_SKILLS {
        assert!(skills.contains(&meta_skill.to_string()));
    }
    assert!(skills.contains(&"ai-product-partner".to_string()));
}

#[test]
fn test_resolve_preset_with_overlays() {
    let manifest = load_presets(library_presets_path()).unwrap();

    let web_skills = manifest.resolve_preset("web").unwrap();
    let qa_skills = manifest.resolve_preset("qa").unwrap();

    let combined = manifest
        .resolve_preset_with_overlays("web", &["qa"])
        .unwrap();

    // Must contain all web skills
    for s in &web_skills {
        assert!(combined.contains(s));
    }
    // Must contain all qa skills
    for s in &qa_skills {
        assert!(combined.contains(s));
    }
    // Must contain all meta-skills
    for &meta in &ALWAYS_ACTIVE_META_SKILLS {
        assert!(combined.contains(&meta.to_string()));
    }
}

#[test]
fn test_all_referenced_skills_exist_in_data_library() {
    let manifest = load_presets(library_presets_path()).unwrap();
    let library_dir = library_skills_dir();
    assert!(library_dir.exists(), "Library dir must exist");

    let mut missing_skills = Vec::new();

    for &preset_name in &REQUIRED_PRESETS {
        let skills = manifest.resolve_preset(preset_name).unwrap();
        for skill in skills {
            let skill_dir = library_dir.join(&skill);
            if !skill_dir.exists() {
                missing_skills.push((preset_name.to_string(), skill));
            }
        }
    }

    assert!(
        missing_skills.is_empty(),
        "Referenced skills missing from data/skills-library: {:?}",
        missing_skills
    );
}

#[test]
fn test_validation_rejects_missing_meta_skill() {
    let yaml = r#"
version: "3.0"
always_active:
  - agent-preset-conductor
  - grill-me
  - systematic-debugging
  - verification-before-completion
  # missing concise-planning
presets:
  web: { name: "Web", skills: [] }
  backend: { name: "Backend", skills: [] }
  devops: { name: "Devops", skills: [] }
  appsec: { name: "Appsec", skills: [] }
  qa: { name: "QA", skills: [] }
  ai-agents: { name: "AI", skills: [] }
  web-research: { name: "Web Res", skills: [] }
  reverse-engineering: { name: "RE", skills: [] }
  product: { name: "Product", skills: [] }
  orchestrator: { name: "Orch", skills: [] }
  systems: { name: "Systems", skills: [] }
  vst3: { name: "VST3", skills: [] }
  hardware: { name: "HW", skills: [] }
  media: { name: "Media", skills: [] }
  meta-dev: { name: "Meta", skills: [] }
  minimal: { name: "Min", skills: [] }
"#;

    let res = load_presets_from_str(yaml);
    assert!(res.is_err());
    match res.unwrap_err() {
        CoreError::MissingAlwaysActiveMetaSkill(skill) => {
            assert_eq!(skill, "concise-planning");
        }
        other => panic!("Expected MissingAlwaysActiveMetaSkill, got: {:?}", other),
    }
}

#[test]
fn test_validation_rejects_retired_all_preset() {
    let yaml = r#"
version: "3.0"
always_active:
  - agent-preset-conductor
  - grill-me
  - systematic-debugging
  - verification-before-completion
  - concise-planning
presets:
  all: { name: "Legacy All", skills: [] }
  web: { name: "Web", skills: [] }
  backend: { name: "Backend", skills: [] }
  devops: { name: "Devops", skills: [] }
  appsec: { name: "Appsec", skills: [] }
  qa: { name: "QA", skills: [] }
  ai-agents: { name: "AI", skills: [] }
  web-research: { name: "Web Res", skills: [] }
  reverse-engineering: { name: "RE", skills: [] }
  product: { name: "Product", skills: [] }
  orchestrator: { name: "Orch", skills: [] }
  systems: { name: "Systems", skills: [] }
  vst3: { name: "VST3", skills: [] }
  hardware: { name: "HW", skills: [] }
  media: { name: "Media", skills: [] }
  meta-dev: { name: "Meta", skills: [] }
  minimal: { name: "Min", skills: [] }
"#;

    let res = load_presets_from_str(yaml);
    assert!(res.is_err());
    match res.unwrap_err() {
        CoreError::DisallowedPreset(name) => {
            assert_eq!(name, "all");
        }
        other => panic!("Expected DisallowedPreset('all'), got: {:?}", other),
    }
}

#[test]
fn test_validation_rejects_missing_required_preset() {
    let yaml = r#"
version: "3.0"
always_active:
  - agent-preset-conductor
  - grill-me
  - systematic-debugging
  - verification-before-completion
  - concise-planning
presets:
  web: { name: "Web", skills: [] }
  backend: { name: "Backend", skills: [] }
  devops: { name: "Devops", skills: [] }
  appsec: { name: "Appsec", skills: [] }
  qa: { name: "QA", skills: [] }
  ai-agents: { name: "AI", skills: [] }
  web-research: { name: "Web Res", skills: [] }
  reverse-engineering: { name: "RE", skills: [] }
  product: { name: "Product", skills: [] }
  orchestrator: { name: "Orch", skills: [] }
  # missing systems, vst3, hardware, media, meta-dev, minimal
"#;

    let res = load_presets_from_str(yaml);
    assert!(res.is_err());
    match res.unwrap_err() {
        CoreError::InvalidPresetCount { expected, found } => {
            assert_eq!(expected, 16);
            assert_eq!(found, 10);
        }
        other => panic!("Expected InvalidPresetCount, got: {:?}", other),
    }
}

#[test]
fn test_resolve_unknown_preset_returns_error() {
    let manifest = load_presets(library_presets_path()).unwrap();
    let res = manifest.resolve_preset("nonexistent_preset");
    assert!(res.is_err());
    match res.unwrap_err() {
        CoreError::PresetNotFound(name) => assert_eq!(name, "nonexistent_preset"),
        other => panic!("Expected PresetNotFound, got: {:?}", other),
    }
}
