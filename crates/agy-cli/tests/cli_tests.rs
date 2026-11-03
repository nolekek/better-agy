use agy_cli::{run_preset_list, run_preset_switch, run_vault_add, run_vault_init, run_vault_list};
use std::path::PathBuf;
use tempfile::tempdir;

fn find_presets_file() -> PathBuf {
    let candidates = [
        PathBuf::from("data/skills-library/presets.yaml"),
        PathBuf::from("../../data/skills-library/presets.yaml"),
        PathBuf::from("../data/skills-library/presets.yaml"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    PathBuf::from("data/skills-library/presets.yaml")
}

fn find_library_dir() -> PathBuf {
    let candidates = [
        PathBuf::from("data/skills-library"),
        PathBuf::from("../../data/skills-library"),
        PathBuf::from("../data/skills-library"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    PathBuf::from("data/skills-library")
}

#[tokio::test]
async fn test_cli_preset_list() {
    let presets_file = find_presets_file();
    let res = run_preset_list(Some(&presets_file));
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_cli_preset_switch() {
    let temp = tempdir().unwrap();
    let ws_path = temp.path();

    let presets_file = find_presets_file();
    let lib_dir = find_library_dir();

    let res = run_preset_switch("minimal", ws_path, Some(&lib_dir), Some(&presets_file));
    assert!(res.is_ok());

    let projected_skills_dir = ws_path.join(".gemini").join("skills");
    assert!(projected_skills_dir.exists());

    // 5 ALWAYS_ACTIVE skills must be present
    let always_active = [
        "agent-preset-conductor",
        "grill-me",
        "systematic-debugging",
        "verification-before-completion",
        "concise-planning",
    ];

    for skill in &always_active {
        assert!(
            projected_skills_dir.join(skill).exists(),
            "Skill {} must exist in projected workspace",
            skill
        );
    }
}

#[tokio::test]
async fn test_cli_vault_crud() {
    let temp = tempdir().unwrap();
    let vault_path = temp.path().join("vault.enc");
    let key_path = temp.path().join("vault.key");

    // 1. Init
    let res = run_vault_init(&vault_path, Some(&key_path)).await;
    assert!(res.is_ok());
    assert!(vault_path.exists());
    assert!(key_path.exists());

    // 2. Add
    let res_add = run_vault_add(&vault_path, Some(&key_path), "dev@company.com", "mock_refresh_token_xyz").await;
    assert!(res_add.is_ok());

    // 3. List
    let res_list = run_vault_list(&vault_path, Some(&key_path)).await;
    assert!(res_list.is_ok());
}
