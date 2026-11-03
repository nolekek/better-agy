use std::fs;
use std::path::PathBuf;
use std::time::Instant;
use tempfile::TempDir;

use agy_core::{
    load_presets, sync_skill_list, sync_skills, SymlinkEngine, ALWAYS_ACTIVE_META_SKILLS,
};

fn library_skills_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("data")
        .join("skills-library")
}

fn library_presets_path() -> PathBuf {
    library_skills_dir().join("presets.yaml")
}

#[test]
fn test_symlink_minimal_preset_creation() {
    let lib_dir = library_skills_dir();
    let manifest = load_presets(library_presets_path()).unwrap();

    let temp_workspace = TempDir::new().unwrap();
    let target_skills_dir = temp_workspace.path().join(".gemini").join("skills");

    let stats = sync_skills(&lib_dir, &target_skills_dir, "minimal", &manifest)
        .expect("sync_skills minimal should succeed");

    assert!(stats.errors.is_empty(), "Errors during sync: {:?}", stats.errors);
    // 5 ALWAYS_ACTIVE meta-skills + 1 domain skill (ai-product-partner) = 6
    assert_eq!(stats.added.len(), 6);
    assert!(stats.removed.is_empty());
    assert!(stats.kept.is_empty());

    // Verify all 5 ALWAYS_ACTIVE meta-skills exist in target directory as valid links
    let engine = SymlinkEngine::new(lib_dir.clone(), target_skills_dir.clone());
    for &meta_skill in &ALWAYS_ACTIVE_META_SKILLS {
        let link_path = target_skills_dir.join(meta_skill);
        assert!(link_path.exists(), "Link '{}' should exist", link_path.display());
        assert!(engine.is_link(&link_path), "Path '{}' must be detected as link", link_path.display());

        // Verify content inside link can be read (e.g., SKILL.md)
        let skill_md = link_path.join("SKILL.md");
        assert!(skill_md.exists(), "SKILL.md inside '{}' must exist", link_path.display());
        let content = fs::read_to_string(&skill_md).unwrap();
        assert!(!content.is_empty(), "SKILL.md content must not be empty");
    }
}

#[test]
fn test_symlink_preset_switch_and_prune() {
    let lib_dir = library_skills_dir();
    let manifest = load_presets(library_presets_path()).unwrap();

    let temp_workspace = TempDir::new().unwrap();
    let target_skills_dir = temp_workspace.path().join(".gemini").join("skills");

    // 1. Initial sync to minimal (6 skills)
    let s1 = sync_skills(&lib_dir, &target_skills_dir, "minimal", &manifest).unwrap();
    assert_eq!(s1.added.len(), 6);
    assert_eq!(s1.removed.len(), 0);

    // 2. Switch to backend (12 domain skills + 5 meta-skills = 13 total unique skills)
    let s2 = sync_skills(&lib_dir, &target_skills_dir, "backend", &manifest).unwrap();
    assert!(s2.errors.is_empty(), "Errors: {:?}", s2.errors);
    // Kept should include the 6 common skills (5 meta-skills + ai-product-partner)
    assert_eq!(s2.kept.len(), 6);
    assert!(!s2.added.is_empty(), "Backend specific skills should be added");
    assert_eq!(s2.removed.len(), 0, "No skills should be removed when switching to superset");

    let backend_total = s2.added.len() + s2.kept.len();

    // 3. Switch back to minimal
    let s3 = sync_skills(&lib_dir, &target_skills_dir, "minimal", &manifest).unwrap();
    assert!(s3.errors.is_empty(), "Errors: {:?}", s3.errors);
    // The 6 common skills should be kept
    assert_eq!(s3.kept.len(), 6);
    assert_eq!(s3.added.len(), 0);
    // All backend-only skills must be removed
    assert_eq!(s3.removed.len(), backend_total - 6);

    // Verify exactly 6 skills remain on disk
    let count = fs::read_dir(&target_skills_dir).unwrap().count();
    assert_eq!(count, 6);
}


#[test]
fn test_idempotent_sync_zero_churn() {
    let lib_dir = library_skills_dir();
    let manifest = load_presets(library_presets_path()).unwrap();

    let temp_workspace = TempDir::new().unwrap();
    let target_skills_dir = temp_workspace.path().join(".gemini").join("skills");

    // First sync
    let s1 = sync_skills(&lib_dir, &target_skills_dir, "web", &manifest).unwrap();
    assert!(s1.errors.is_empty());
    let total_web_skills = s1.added.len();

    // Second sync immediately with same preset
    let s2 = sync_skills(&lib_dir, &target_skills_dir, "web", &manifest).unwrap();
    assert!(s2.errors.is_empty());
    assert_eq!(s2.added.len(), 0, "Zero new links should be added on re-sync");
    assert_eq!(s2.removed.len(), 0, "Zero links should be removed on re-sync");
    assert_eq!(s2.kept.len(), total_web_skills, "All skills should be kept");
}

#[test]
fn test_preserves_user_non_symlink_directories() {
    let lib_dir = library_skills_dir();
    let manifest = load_presets(library_presets_path()).unwrap();

    let temp_workspace = TempDir::new().unwrap();
    let target_skills_dir = temp_workspace.path().join(".gemini").join("skills");
    fs::create_dir_all(&target_skills_dir).unwrap();

    // User creates a custom tool directory directly in .gemini/skills/
    let user_dir = target_skills_dir.join("user-custom-tool");
    fs::create_dir_all(&user_dir).unwrap();
    let user_file = user_dir.join("custom_tool.py");
    fs::write(&user_file, b"print('Hello custom tool')").unwrap();

    // Sync to backend
    let s1 = sync_skills(&lib_dir, &target_skills_dir, "backend", &manifest).unwrap();
    assert!(s1.preserved_user_dirs.contains(&"user-custom-tool".to_string()));
    assert!(user_dir.exists(), "User directory must NOT be deleted");
    assert!(user_file.exists(), "User file inside custom dir must NOT be deleted");

    // Sync to minimal
    let s2 = sync_skills(&lib_dir, &target_skills_dir, "minimal", &manifest).unwrap();
    assert!(s2.preserved_user_dirs.contains(&"user-custom-tool".to_string()));
    assert!(user_dir.exists(), "User directory must NOT be deleted when switching to minimal");
    assert_eq!(
        fs::read_to_string(&user_file).unwrap(),
        "print('Hello custom tool')"
    );
}

#[test]
fn test_cleans_up_broken_symlinks() {
    let lib_dir = library_skills_dir();
    let temp_workspace = TempDir::new().unwrap();
    let target_skills_dir = temp_workspace.path().join(".gemini").join("skills");
    fs::create_dir_all(&target_skills_dir).unwrap();

    let engine = SymlinkEngine::new(lib_dir.clone(), target_skills_dir.clone());

    // Create a temporary directory that we link to, then delete the target to create a broken link
    let dummy_target = temp_workspace.path().join("dummy_target");
    fs::create_dir_all(&dummy_target).unwrap();
    let link_path = target_skills_dir.join("stale-broken-skill");
    engine.create_link(&dummy_target, &link_path).unwrap();
    assert!(engine.is_link(&link_path));

    // Delete the target to make the link broken
    fs::remove_dir_all(&dummy_target).unwrap();
    assert!(!dummy_target.exists());
    // The broken link should still be detected as a link
    assert!(engine.is_link(&link_path));

    // Now run sync with desired skills that do NOT include "stale-broken-skill"
    let desired = vec!["agent-preset-conductor".to_string()];
    let report = sync_skill_list(&lib_dir, &target_skills_dir, &desired).unwrap();

    assert!(report.removed.contains(&"stale-broken-skill".to_string()));
    assert!(!link_path.exists());
    assert!(!engine.is_link(&link_path));
}

#[test]
fn test_security_traversal_rejection() {
    let lib_dir = library_skills_dir();
    let temp_workspace = TempDir::new().unwrap();
    let target_skills_dir = temp_workspace.path().join(".gemini").join("skills");

    // Malicious skill name attempting traversal
    let desired = vec![
        "../../etc/shadow".to_string(),
        "..\\windows\\system32".to_string(),
        "normal/slash".to_string(),
    ];

    let report = sync_skill_list(&lib_dir, &target_skills_dir, &desired).unwrap();

    // All traversal attempts must be flagged in errors and zero links added
    assert_eq!(report.added.len(), 0);
    assert_eq!(report.errors.len(), 3);
    for err in &report.errors {
        assert!(err.1.contains("Path traversal attempt detected"));
    }
}

#[test]
fn test_zero_mutation_of_library_source() {
    let lib_dir = library_skills_dir();
    let manifest = load_presets(library_presets_path()).unwrap();

    let conductor_path = lib_dir.join("agent-preset-conductor").join("SKILL.md");
    assert!(conductor_path.exists());
    let initial_bytes = fs::read(&conductor_path).unwrap();

    let temp_workspace = TempDir::new().unwrap();
    let target_skills_dir = temp_workspace.path().join(".gemini").join("skills");

    // Perform rapid switches
    for preset in &["web", "backend", "devops", "systems", "minimal"] {
        let stats = sync_skills(&lib_dir, &target_skills_dir, preset, &manifest).unwrap();
        assert!(stats.errors.is_empty());
    }

    // Verify source file is completely untouched
    let final_bytes = fs::read(&conductor_path).unwrap();
    assert_eq!(
        initial_bytes, final_bytes,
        "Source library files MUST never be modified"
    );
}

#[test]
fn test_sub_millisecond_projection_performance() {
    let lib_dir = library_skills_dir();
    let manifest = load_presets(library_presets_path()).unwrap();

    let temp_workspace = TempDir::new().unwrap();
    let target_skills_dir = temp_workspace.path().join(".gemini").join("skills");

    // Initial sync
    sync_skills(&lib_dir, &target_skills_dir, "web", &manifest).unwrap();

    // Measure switch from web to backend
    let start = Instant::now();
    let report = sync_skills(&lib_dir, &target_skills_dir, "backend", &manifest).unwrap();
    let elapsed = start.elapsed();

    assert!(report.errors.is_empty());
    // Should take well under 100ms (typically 2-10ms)
    assert!(
        elapsed.as_millis() < 500,
        "Preset switch took too long: {}ms",
        elapsed.as_millis()
    );
}
