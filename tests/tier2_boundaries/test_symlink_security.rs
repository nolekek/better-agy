use agy_core::{
    load_presets_from_str, sync_skills, CoreError, SymlinkEngine,
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
      - "react-skill"
  backend:
    skills: []
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

fn setup_lib_and_workspace() -> (tempfile::TempDir, tempfile::TempDir) {
    let lib_tmp = tempdir().unwrap();
    let ws_tmp = tempdir().unwrap();

    let skills = [
        "agent-preset-conductor",
        "grill-me",
        "systematic-debugging",
        "verification-before-completion",
        "concise-planning",
        "react-skill",
    ];

    for s in &skills {
        let p = lib_tmp.path().join(s);
        fs::create_dir_all(&p).unwrap();
        fs::write(p.join("SKILL.md"), "# Skill\n").unwrap();
    }

    (lib_tmp, ws_tmp)
}

#[test]
fn test_f3_preserve_user_created_regular_directory() {
    let (lib_tmp, ws_tmp) = setup_lib_and_workspace();
    let target_skills_dir = ws_tmp.path().join(".gemini").join("skills");
    fs::create_dir_all(&target_skills_dir).unwrap();

    // User manually creates a non-symlink custom tool folder
    let user_folder = target_skills_dir.join("my-custom-unlinked-tool");
    fs::create_dir_all(&user_folder).unwrap();
    fs::write(user_folder.join("custom.py"), "print('important user code')").unwrap();

    let manifest = load_presets_from_str(SAMPLE_PRESETS_YAML).unwrap();

    // Sync skills
    let stats = sync_skills(lib_tmp.path(), &target_skills_dir, "web", &manifest).unwrap();

    // The user folder must NOT be deleted!
    assert!(
        user_folder.exists(),
        "User folder must be preserved across syncs"
    );
    assert!(user_folder.join("custom.py").exists());
    assert!(
        stats.preserved_user_dirs.contains(&"my-custom-unlinked-tool".to_string()),
        "SyncReport should report preserved user directory"
    );
}

#[test]
fn test_f3_cleanup_broken_symlinks() {
    let (lib_tmp, ws_tmp) = setup_lib_and_workspace();
    let target_skills_dir = ws_tmp.path().join(".gemini").join("skills");
    let manifest = load_presets_from_str(SAMPLE_PRESETS_YAML).unwrap();

    // Initial sync
    sync_skills(lib_tmp.path(), &target_skills_dir, "web", &manifest).unwrap();

    // Delete one target in the library so link becomes broken / dangling
    let react_lib = lib_tmp.path().join("react-skill");
    fs::remove_dir_all(&react_lib).unwrap();

    // Re-sync with minimal (which shouldn't include react-skill)
    let stats = sync_skills(lib_tmp.path(), &target_skills_dir, "minimal", &manifest)
        .expect("Sync should cleanly clean up broken link");

    assert!(stats.removed.contains(&"react-skill".to_string()));
    assert!(!target_skills_dir.join("react-skill").exists());
}

#[test]
fn test_f2_reject_disallowed_all_preset() {
    let yaml_with_all = r#"
version: "3.0"
always_active:
  - "agent-preset-conductor"
  - "grill-me"
  - "systematic-debugging"
  - "verification-before-completion"
  - "concise-planning"
presets:
  all:
    skills: ["everything"]
  web:
    skills: []
  backend:
    skills: []
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

    let res = load_presets_from_str(yaml_with_all);
    assert!(res.is_err(), "Manifest with 'all' preset must be rejected");
    match res.unwrap_err() {
        CoreError::DisallowedPreset(_) => {}
        other => panic!("Expected DisallowedPreset, got: {:?}", other),
    }
}

#[test]
fn test_f3_reject_directory_traversal_attack() {
    let (lib_tmp, ws_tmp) = setup_lib_and_workspace();
    let target_skills_dir = ws_tmp.path().join(".gemini").join("skills");

    let engine = SymlinkEngine::new(
        lib_tmp.path().to_path_buf(),
        target_skills_dir.clone(),
    );

    // Attempt to sync a skill name with directory traversal
    let traversal_skill = "../../etc/passwd".to_string();
    let stats = engine
        .sync_skills(std::slice::from_ref(&traversal_skill))
        .expect("Engine should handle traversal error safely");

    // Traversal attempt must be captured in report.errors, never created!
    assert!(
        stats.errors.iter().any(|(s, _)| s == &traversal_skill),
        "Traversal skill must produce an error"
    );
    assert!(!target_skills_dir.join("../../etc/passwd").exists());
}

#[test]
fn test_f3_auto_creates_missing_target_skills_dir() {
    let (lib_tmp, ws_tmp) = setup_lib_and_workspace();
    let non_existent_skills_dir = ws_tmp
        .path()
        .join("nested_workspace")
        .join(".gemini")
        .join("skills");

    assert!(!non_existent_skills_dir.exists());

    let manifest = load_presets_from_str(SAMPLE_PRESETS_YAML).unwrap();
    let stats = sync_skills(lib_tmp.path(), &non_existent_skills_dir, "web", &manifest)
        .expect("Sync should create target directory");

    assert!(non_existent_skills_dir.exists());
    assert_eq!(stats.added.len(), 6); // 5 meta + 1 react
}
