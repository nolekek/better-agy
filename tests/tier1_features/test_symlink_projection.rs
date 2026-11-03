use agy_core::{
    load_presets_from_str, sync_skills, ALWAYS_ACTIVE_META_SKILLS,
};
use std::fs;
use tempfile::tempdir;

const SAMPLE_PRESETS_YAML: &str = r#"
version: "3.0"
always_active:
  - "agent-preset-conductor"
  - "grill-me"
  - "systematic-debugging"
  - "verification-before-completion"
  - "concise-planning"
presets:
  web:
    skills:
      - "nextjs-fullstack-guru"
      - "react-best-practices"
  backend:
    skills:
      - "postgres-dba-optimizer"
      - "database-design"
  devops:
    skills: []
  appsec:
    skills: []
  qa:
    skills: []
  ai-agents:
    skills: []
  web-research:
    skills: []
  reverse-engineering:
    skills: []
  product:
    skills: []
  orchestrator:
    skills: []
  systems:
    skills: []
  vst3:
    skills: []
  hardware:
    skills: []
  media:
    skills: []
  meta-dev:
    skills: []
  minimal:
    skills: []
"#;

fn create_mock_library(library_dir: &std::path::Path) {
    let all_skills = [
        "agent-preset-conductor",
        "grill-me",
        "systematic-debugging",
        "verification-before-completion",
        "concise-planning",
        "nextjs-fullstack-guru",
        "react-best-practices",
        "postgres-dba-optimizer",
        "database-design",
    ];

    for skill in &all_skills {
        let skill_dir = library_dir.join(skill);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), format!("# Skill {}\n", skill)).unwrap();
    }
}

#[test]
fn test_f3_sync_skills_initial_projection() {
    let lib_tmp = tempdir().unwrap();
    create_mock_library(lib_tmp.path());

    let ws_tmp = tempdir().unwrap();
    let target_skills_dir = ws_tmp.path().join(".gemini").join("skills");

    let manifest = load_presets_from_str(SAMPLE_PRESETS_YAML).unwrap();

    let stats = sync_skills(lib_tmp.path(), &target_skills_dir, "web", &manifest)
        .expect("Sync skills should succeed");

    // 5 meta-skills + 2 web skills = 7 added
    assert_eq!(stats.added.len(), 7);
    assert_eq!(stats.removed.len(), 0);
    assert!(target_skills_dir.join("agent-preset-conductor").exists());
    assert!(target_skills_dir.join("nextjs-fullstack-guru").exists());
}

#[test]
fn test_f3_symlink_target_integrity() {
    let lib_tmp = tempdir().unwrap();
    create_mock_library(lib_tmp.path());

    let ws_tmp = tempdir().unwrap();
    let target_skills_dir = ws_tmp.path().join(".gemini").join("skills");

    let manifest = load_presets_from_str(SAMPLE_PRESETS_YAML).unwrap();
    sync_skills(lib_tmp.path(), &target_skills_dir, "web", &manifest).unwrap();

    let meta_link = target_skills_dir.join("agent-preset-conductor");
    assert!(meta_link.exists());
    let skill_md = meta_link.join("SKILL.md");
    assert!(skill_md.exists());
    let content = fs::read_to_string(skill_md).unwrap();
    assert!(content.contains("agent-preset-conductor"));
}

#[test]
fn test_f3_switch_preset_prunes_stale_and_adds_new() {
    let lib_tmp = tempdir().unwrap();
    create_mock_library(lib_tmp.path());

    let ws_tmp = tempdir().unwrap();
    let target_skills_dir = ws_tmp.path().join(".gemini").join("skills");

    let manifest = load_presets_from_str(SAMPLE_PRESETS_YAML).unwrap();

    // 1. Project 'web'
    sync_skills(lib_tmp.path(), &target_skills_dir, "web", &manifest).unwrap();
    assert!(target_skills_dir.join("nextjs-fullstack-guru").exists());

    // 2. Switch to 'backend'
    let stats = sync_skills(lib_tmp.path(), &target_skills_dir, "backend", &manifest).unwrap();

    // Stale web skills removed
    assert!(stats.removed.contains(&"nextjs-fullstack-guru".to_string()));
    assert!(stats.removed.contains(&"react-best-practices".to_string()));
    assert!(!target_skills_dir.join("nextjs-fullstack-guru").exists());

    // New backend skills added
    assert!(stats.added.contains(&"postgres-dba-optimizer".to_string()));
    assert!(target_skills_dir.join("postgres-dba-optimizer").exists());

    // Meta skills kept
    for &meta in &ALWAYS_ACTIVE_META_SKILLS {
        assert!(stats.kept.contains(&meta.to_string()));
        assert!(target_skills_dir.join(meta).exists());
    }
}

#[test]
fn test_f3_meta_skills_preserved_across_preset_switches() {
    let lib_tmp = tempdir().unwrap();
    create_mock_library(lib_tmp.path());

    let ws_tmp = tempdir().unwrap();
    let target_skills_dir = ws_tmp.path().join(".gemini").join("skills");

    let manifest = load_presets_from_str(SAMPLE_PRESETS_YAML).unwrap();

    sync_skills(lib_tmp.path(), &target_skills_dir, "web", &manifest).unwrap();
    sync_skills(lib_tmp.path(), &target_skills_dir, "backend", &manifest).unwrap();
    let stats = sync_skills(lib_tmp.path(), &target_skills_dir, "minimal", &manifest).unwrap();

    for &meta in &ALWAYS_ACTIVE_META_SKILLS {
        assert!(
            stats.kept.contains(&meta.to_string()),
            "Meta skill {} must be kept",
            meta
        );
        assert!(target_skills_dir.join(meta).exists());
    }
}

#[test]
fn test_f3_sub_millisecond_projection_performance() {
    let lib_tmp = tempdir().unwrap();
    create_mock_library(lib_tmp.path());

    let ws_tmp = tempdir().unwrap();
    let target_skills_dir = ws_tmp.path().join(".gemini").join("skills");

    let manifest = load_presets_from_str(SAMPLE_PRESETS_YAML).unwrap();

    let stats = sync_skills(lib_tmp.path(), &target_skills_dir, "web", &manifest).unwrap();

    // Must be fast (< 500ms on Windows NTFS filesystem handle latency, sub-millisecond on Linux tmpfs/ext4)
    assert!(
        stats.duration_ms < 500,
        "Sync skills took too long: {}ms",
        stats.duration_ms
    );
}
