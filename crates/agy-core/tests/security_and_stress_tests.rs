use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Barrier};
use std::thread;
use tempfile::TempDir;

use agy_core::{
    load_presets, load_presets_from_str, sync_skill_list, sync_skills,
    CoreError, SymlinkEngine, SymlinkError, ALWAYS_ACTIVE_META_SKILLS,
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

// =========================================================================
// CHALLENGE 1: Path Traversal Security
// =========================================================================

#[test]
fn test_challenge_path_traversal_variations() {
    let lib_dir = library_skills_dir();
    let temp_workspace = TempDir::new().unwrap();
    let target_skills_dir = temp_workspace.path().join(".gemini").join("skills");

    let traversal_payloads = vec![
        "../",
        "..\\",
        "../../etc/passwd",
        "..\\..\\Windows\\System32",
        "normal/nested",
        "normal\\nested",
        "..",
        "nested/../../etc",
        "nested\\..\\..\\Windows",
        "C:\\Windows",
        "C:/Windows",
        "/etc/shadow",
        "/root/.ssh/id_rsa",
        "\\\\localhost\\c$\\Windows",
    ];

    for payload in traversal_payloads {
        let desired = vec![payload.to_string()];
        let report = sync_skill_list(&lib_dir, &target_skills_dir, &desired)
            .expect("sync_skill_list should not panic on traversal attempt");

        assert_eq!(
            report.added.len(),
            0,
            "Traversal payload '{}' should NEVER result in an added link!",
            payload
        );

        assert!(
            !report.errors.is_empty(),
            "Traversal payload '{}' MUST be rejected with an error!",
            payload
        );

        let has_security_err = report.errors.iter().any(|(skill, err)| {
            skill == payload
                && (err.contains("Path traversal attempt detected")
                    || err.contains("Security violation")
                    || err.contains("does not exist"))
        });

        assert!(
            has_security_err,
            "Payload '{}' did not trigger proper security rejection: {:?}",
            payload, report.errors
        );
    }
}

#[test]
fn test_challenge_single_dot_payload() {
    let lib_dir = library_skills_dir();
    let temp_workspace = TempDir::new().unwrap();
    let target_skills_dir = temp_workspace.path().join(".gemini").join("skills");

    // Single dot refers to target directory itself or library directory itself
    let desired = vec![".".to_string()];
    let report = sync_skill_list(&lib_dir, &target_skills_dir, &desired);

    // It should either report an error or handle safely without corrupting target_skills_dir
    if let Ok(stats) = report {
        // Must not create a circular link or wipe target_skills_dir
        assert!(
            target_skills_dir.exists(),
            "target_skills_dir must remain a valid directory"
        );
        println!("Single dot report: {:?}", stats);
    }
}

#[test]
fn test_challenge_symlink_escape_within_library() {
    let temp_lib = TempDir::new().unwrap();
    let lib_dir = temp_lib.path().join("skills-library");
    fs::create_dir_all(&lib_dir).unwrap();

    let temp_workspace = TempDir::new().unwrap();
    let target_skills_dir = temp_workspace.path().join(".gemini").join("skills");

    // Create an outside secret directory
    let outside_secret_dir = temp_lib.path().join("outside_secret");
    fs::create_dir_all(&outside_secret_dir).unwrap();
    fs::write(outside_secret_dir.join("secret.key"), "PRIVATE_KEY_DATA").unwrap();

    // Inside library_dir, create a symlink or junction pointing to outside_secret_dir
    let rogue_skill_dir = lib_dir.join("rogue-escape-skill");
    #[cfg(windows)]
    {
        let abs_outside = outside_secret_dir.canonicalize().unwrap();
        let _ = junction::create(&abs_outside, &rogue_skill_dir);
    }
    #[cfg(not(windows))]
    {
        let abs_outside = outside_secret_dir.canonicalize().unwrap();
        let _ = std::os::unix::fs::symlink(&abs_outside, &rogue_skill_dir);
    }

    if rogue_skill_dir.exists() {
        // Now try to project this skill
        let desired = vec!["rogue-escape-skill".to_string()];
        let report = sync_skill_list(&lib_dir, &target_skills_dir, &desired)
            .expect("Engine should handle rogue skill without panicking");

        // The rogue skill escapes library directory, so it MUST be rejected!
        assert_eq!(
            report.added.len(),
            0,
            "Rogue symlink escaping library must NOT be added!"
        );
        assert!(
            report.errors.iter().any(|(s, e)| s == "rogue-escape-skill"
                && e.contains("escapes library directory")),
            "Expected 'escapes library directory' error, got: {:?}",
            report.errors
        );
    }
}

// =========================================================================
// CHALLENGE 2: User File Preservation
// =========================================================================

#[test]
fn test_challenge_user_directory_and_file_preservation_across_rapid_switches() {
    let lib_dir = library_skills_dir();
    let manifest = load_presets(library_presets_path()).unwrap();

    let temp_workspace = TempDir::new().unwrap();
    let target_skills_dir = temp_workspace.path().join(".gemini").join("skills");
    fs::create_dir_all(&target_skills_dir).unwrap();

    // 1. Create user directory with multiple files and nested subdirectories
    let user_custom_dir = target_skills_dir.join("user-custom-agent");
    fs::create_dir_all(user_custom_dir.join("submodules")).unwrap();

    let file1_path = user_custom_dir.join("SKILL.md");
    let file2_path = user_custom_dir.join("submodules").join("helper.py");
    let file1_content = "# My Custom User Agent\nDo not delete me!";
    let file2_content = "def helper(): return 42\n";

    fs::write(&file1_path, file1_content).unwrap();
    fs::write(&file2_path, file2_content).unwrap();

    // 2. Create regular user files directly in target_skills_dir root
    let root_file_path = target_skills_dir.join("README-USER.txt");
    let root_file_content = "Important user notes for skills directory";
    fs::write(&root_file_path, root_file_content).unwrap();

    // 3. Perform 50 rapid switches across diverse presets
    let presets_cycle = [
        "web",
        "backend",
        "devops",
        "qa",
        "ai-agents",
        "web-research",
        "reverse-engineering",
        "product",
        "orchestrator",
        "systems",
        "vst3",
        "hardware",
        "media",
        "meta-dev",
        "minimal",
        "appsec",
    ];

    for i in 0..50 {
        let preset = presets_cycle[i % presets_cycle.len()];
        let report = sync_skills(&lib_dir, &target_skills_dir, preset, &manifest)
            .unwrap_or_else(|e| panic!("Sync failed at switch {} to '{}': {:?}", i, preset, e));

        assert!(
            report.errors.is_empty(),
            "Switch {} to '{}' encountered errors: {:?}",
            i, preset, report.errors
        );

        // Verification: User directory MUST be reported in preserved_user_dirs
        assert!(
            report.preserved_user_dirs.contains(&"user-custom-agent".to_string()),
            "user-custom-agent must be preserved on switch {} to '{}'",
            i, preset
        );

        // Verification: User directory still exists on disk
        assert!(user_custom_dir.exists());
        assert!(user_custom_dir.is_dir());

        // Verification: File contents remain pristine and untouched
        assert_eq!(fs::read_to_string(&file1_path).unwrap(), file1_content);
        assert_eq!(fs::read_to_string(&file2_path).unwrap(), file2_content);

        // Verification: Root user file still exists and is untouched
        assert!(root_file_path.exists());
        assert_eq!(fs::read_to_string(&root_file_path).unwrap(), root_file_content);
    }
}

#[test]
fn test_challenge_user_directory_name_collision_with_skill() {
    let lib_dir = library_skills_dir();
    let manifest = load_presets(library_presets_path()).unwrap();

    let temp_workspace = TempDir::new().unwrap();
    let target_skills_dir = temp_workspace.path().join(".gemini").join("skills");
    fs::create_dir_all(&target_skills_dir).unwrap();

    // User creates a directory whose name matches a skill that is in the "backend" preset
    let colliding_dir = target_skills_dir.join("systematic-debugging");
    fs::create_dir_all(&colliding_dir).unwrap();
    let secret_file = colliding_dir.join("user_local_custom_debug.txt");
    fs::write(&secret_file, "LOCAL CUSTOM DEBUG SCRIPT - DO NOT OVERWRITE").unwrap();

    // Now sync to minimal or backend, which both require "systematic-debugging"
    let report = sync_skills(&lib_dir, &target_skills_dir, "minimal", &manifest).unwrap();

    // The engine MUST preserve the user directory, NOT overwrite it with a link!
    assert!(
        report.preserved_user_dirs.contains(&"systematic-debugging".to_string()),
        "Colliding user directory must be preserved!"
    );

    // It should log an error or note that regular directory exists at target location
    assert!(
        report.errors.iter().any(|(s, e)| s == "systematic-debugging"
            && e.contains("regular user directory exists at target location")),
        "Expected error warning about regular user directory collision, got: {:?}",
        report.errors
    );

    // Check disk: user's file is intact!
    assert!(secret_file.exists());
    assert_eq!(
        fs::read_to_string(&secret_file).unwrap(),
        "LOCAL CUSTOM DEBUG SCRIPT - DO NOT OVERWRITE"
    );

    // Engine is_link must return false (it is NOT a symlink)
    let engine = SymlinkEngine::new(lib_dir.clone(), target_skills_dir.clone());
    assert!(!engine.is_link(&colliding_dir));
}

// =========================================================================
// CHALLENGE 3: Race Conditions, Broken Links, Restricted Permissions
// =========================================================================

#[test]
fn test_challenge_broken_symlinks_repair_and_cleanup() {
    let lib_dir = library_skills_dir();
    let manifest = load_presets(library_presets_path()).unwrap();

    let temp_workspace = TempDir::new().unwrap();
    let target_skills_dir = temp_workspace.path().join(".gemini").join("skills");
    fs::create_dir_all(&target_skills_dir).unwrap();

    let engine = SymlinkEngine::new(lib_dir.clone(), target_skills_dir.clone());

    // 1. Create a dummy external target and link to it
    let dummy_target = temp_workspace.path().join("temporary_target");
    fs::create_dir_all(&dummy_target).unwrap();
    let dead_link = target_skills_dir.join("broken-link-1");
    engine.create_link(&dummy_target, &dead_link).unwrap();

    // 2. Destroy the target so the link is broken / dangling
    fs::remove_dir_all(&dummy_target).unwrap();
    assert!(!dummy_target.exists());
    assert!(engine.is_link(&dead_link));

    // 3. Sync to "web" (which does not contain "broken-link-1")
    let report = sync_skills(&lib_dir, &target_skills_dir, "web", &manifest).unwrap();

    // Dead link must have been safely detected and removed!
    assert!(
        report.removed.contains(&"broken-link-1".to_string()),
        "Dead link should be in removed list: {:?}",
        report.removed
    );
    assert!(!engine.is_link(&dead_link));
}

#[test]
fn test_challenge_concurrent_sync_calls_no_panics() {
    let lib_dir = Arc::new(library_skills_dir());
    let manifest = Arc::new(load_presets(library_presets_path()).unwrap());

    let temp_workspace = TempDir::new().unwrap();
    let target_skills_dir = Arc::new(temp_workspace.path().join(".gemini").join("skills"));
    fs::create_dir_all(&*target_skills_dir).unwrap();

    let num_threads = 6;
    let barrier = Arc::new(Barrier::new(num_threads));
    let mut handles = Vec::new();

    let presets = ["web", "backend", "devops", "qa", "systems", "minimal"];

    for i in 0..num_threads {
        let b = Arc::clone(&barrier);
        let l = Arc::clone(&lib_dir);
        let m = Arc::clone(&manifest);
        let t = Arc::clone(&target_skills_dir);
        let preset_name = presets[i % presets.len()].to_string();

        let handle = thread::spawn(move || {
            b.wait(); // All threads synchronize to hit at the exact same instant
            for _ in 0..10 {
                let _ = sync_skills(&*l, &*t, &preset_name, &m);
            }
        });
        handles.push(handle);
    }

    // Join all threads - no thread should panic!
    for h in handles {
        h.join().expect("Concurrent sync thread panicked!");
    }

    let conductor_path = target_skills_dir.join("agent-preset-conductor");
    let engine = SymlinkEngine::new((*lib_dir).clone(), (*target_skills_dir).clone());
    let is_link = engine.is_link(&conductor_path);
    let is_dir = conductor_path.is_dir();
    let entries_count = if is_dir {
        fs::read_dir(&conductor_path).map(|r| r.count()).unwrap_or(0)
    } else {
        0
    };
    println!(
        "Conductor post-concurrency state: exists={}, is_dir={}, is_link={}, entries={}",
        conductor_path.exists(), is_dir, is_link, entries_count
    );

    // Final single-threaded sync
    let final_stats = sync_skills(&*lib_dir, &*target_skills_dir, "web", &manifest)
        .expect("Final sync call must not panic");
    assert!(
        final_stats.errors.is_empty(),
        "Concurrency race condition must be recovered: zero errors expected, got: {:?}",
        final_stats.errors
    );
    let expected_web_skills = manifest.resolve_preset("web").unwrap();
    assert_eq!(
        final_stats.added.len() + final_stats.kept.len(),
        expected_web_skills.len(),
        "All web skills must be cleanly established"
    );
}

#[test]
fn test_challenge_missing_library_directory_error() {
    let temp_workspace = TempDir::new().unwrap();
    let missing_lib = temp_workspace.path().join("does_not_exist");
    let target_skills_dir = temp_workspace.path().join(".gemini").join("skills");

    let err = sync_skill_list(&missing_lib, &target_skills_dir, &["web".to_string()]);
    assert!(err.is_err());
    match err.unwrap_err() {
        CoreError::Symlink(SymlinkError::PathNotFound(p)) => {
            assert_eq!(p, missing_lib);
        }
        other => panic!("Expected PathNotFound error, got: {:?}", other),
    }
}

// =========================================================================
// CHALLENGE 4: Validation of 16 Presets & 5 ALWAYS_ACTIVE Meta-Skills
// =========================================================================

#[test]
fn test_challenge_yaml_malformed_syntax() {
    let bad_yamls = vec![
        "{ this is not valid yaml",
        "version: 3.0\nalways_active: [unclosed bracket",
        "::: bad yaml :::",
        "version: 3.0\npresets:\n  web: *undefined_alias",
    ];

    for bad in bad_yamls {
        let res = load_presets_from_str(bad);
        assert!(res.is_err(), "Malformed YAML must be rejected: {}", bad);
        match res.unwrap_err() {
            CoreError::Yaml(_) => {} // expected
            other => panic!("Expected CoreError::Yaml, got: {:?}", other),
        }
    }
}

#[test]
fn test_challenge_missing_each_always_active_meta_skill_individually() {
    for missing_meta in &ALWAYS_ACTIVE_META_SKILLS {
        let remaining_meta: Vec<&str> = ALWAYS_ACTIVE_META_SKILLS
            .iter()
            .copied()
            .filter(|&m| m != *missing_meta)
            .collect();

        let yaml = format!(
            r#"
version: "3.0"
always_active:
{}
presets:
  web: []
  backend: []
  devops: []
  appsec: []
  qa: []
  ai-agents: []
  web-research: []
  reverse-engineering: []
  product: []
  orchestrator: []
  systems: []
  vst3: []
  hardware: []
  media: []
  meta-dev: []
  minimal: []
"#,
            remaining_meta
                .iter()
                .map(|m| format!("  - {}", m))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let res = load_presets_from_str(&yaml);
        assert!(
            res.is_err(),
            "Manifest missing meta-skill '{}' must be rejected",
            missing_meta
        );

        match res.unwrap_err() {
            CoreError::MissingAlwaysActiveMetaSkill(skill) => {
                assert_eq!(skill, *missing_meta);
            }
            other => panic!("Expected MissingAlwaysActiveMetaSkill, got: {:?}", other),
        }
    }
}

#[test]
fn test_challenge_retired_all_preset_rejected_regardless_of_count() {
    // 16 presets, but one of them is "all" instead of "minimal"
    let yaml = r#"
version: "3.0"
always_active:
  - agent-preset-conductor
  - grill-me
  - systematic-debugging
  - verification-before-completion
  - concise-planning
presets:
  all: []
  web: []
  backend: []
  devops: []
  appsec: []
  qa: []
  ai-agents: []
  web-research: []
  reverse-engineering: []
  product: []
  orchestrator: []
  systems: []
  vst3: []
  hardware: []
  media: []
  meta-dev: []
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
fn test_challenge_wrong_preset_name_rejected() {
    // 16 presets, but "web" is misspelled as "web-frontend"
    let yaml = r#"
version: "3.0"
always_active:
  - agent-preset-conductor
  - grill-me
  - systematic-debugging
  - verification-before-completion
  - concise-planning
presets:
  web-frontend: []
  backend: []
  devops: []
  appsec: []
  qa: []
  ai-agents: []
  web-research: []
  reverse-engineering: []
  product: []
  orchestrator: []
  systems: []
  vst3: []
  hardware: []
  media: []
  meta-dev: []
  minimal: []
"#;

    let res = load_presets_from_str(yaml);
    assert!(res.is_err());
    match res.unwrap_err() {
        CoreError::MissingDomainPreset(name) => {
            assert_eq!(name, "web");
        }
        other => panic!("Expected MissingDomainPreset('web'), got: {:?}", other),
    }
}

#[test]
fn test_challenge_count_mismatches_rejected() {
    // 17 presets
    let yaml_17 = r#"
version: "3.0"
always_active:
  - agent-preset-conductor
  - grill-me
  - systematic-debugging
  - verification-before-completion
  - concise-planning
presets:
  web: []
  backend: []
  devops: []
  appsec: []
  qa: []
  ai-agents: []
  web-research: []
  reverse-engineering: []
  product: []
  orchestrator: []
  systems: []
  vst3: []
  hardware: []
  media: []
  meta-dev: []
  minimal: []
  extra-custom-preset: []
"#;

    let res = load_presets_from_str(yaml_17);
    assert!(res.is_err());
    match res.unwrap_err() {
        CoreError::InvalidPresetCount { expected, found } => {
            assert_eq!(expected, 16);
            assert_eq!(found, 17);
        }
        other => panic!("Expected InvalidPresetCount, got: {:?}", other),
    }
}

#[test]
fn test_challenge_manifest_validation_traversal_gap() {
    // Presets manifest containing a path traversal payload in preset skills
    let yaml = r#"
version: "3.0"
always_active:
  - agent-preset-conductor
  - grill-me
  - systematic-debugging
  - verification-before-completion
  - concise-planning
presets:
  web:
    - "../../etc/shadow"
  backend: []
  devops: []
  appsec: []
  qa: []
  ai-agents: []
  web-research: []
  reverse-engineering: []
  product: []
  orchestrator: []
  systems: []
  vst3: []
  hardware: []
  media: []
  meta-dev: []
  minimal: []
"#;

    // Notice: load_presets_from_str validates the manifest!
    let res = load_presets_from_str(yaml);
    assert!(res.is_err(), "Manifest validation must reject traversal payload");
    match res {
        Err(CoreError::InvalidSkillIdentifier(skill)) => {
            assert_eq!(skill, "../../etc/shadow");
        }
        other => {
            panic!("Expected CoreError::InvalidSkillIdentifier, got: {:?}", other);
        }
    }
}

#[test]
fn test_challenge_dangling_link_when_library_target_deleted() {
    let temp_lib = TempDir::new().unwrap();
    let lib_dir = temp_lib.path().join("skills-library");
    fs::create_dir_all(&lib_dir).unwrap();

    let temp_workspace = TempDir::new().unwrap();
    let target_skills_dir = temp_workspace.path().join(".gemini").join("skills");

    // 1. Create a skill in library
    let skill_a_dir = lib_dir.join("ephemeral-skill");
    fs::create_dir_all(&skill_a_dir).unwrap();
    fs::write(skill_a_dir.join("SKILL.md"), "Ephemeral skill").unwrap();

    let engine = SymlinkEngine::new(lib_dir.clone(), target_skills_dir.clone());

    // 2. Initial sync
    let desired = vec!["ephemeral-skill".to_string()];
    let report1 = sync_skill_list(&lib_dir, &target_skills_dir, &desired).unwrap();
    assert_eq!(report1.added, vec!["ephemeral-skill".to_string()]);

    let link_path = target_skills_dir.join("ephemeral-skill");
    assert!(engine.is_link(&link_path));

    // 3. Target in library is deleted
    fs::remove_dir_all(&skill_a_dir).unwrap();
    assert!(!skill_a_dir.exists());

    // 4. Re-sync with desired containing the now-deleted skill
    let report2 = sync_skill_list(&lib_dir, &target_skills_dir, &desired).unwrap();

    // Observe: report2 gets an error because target does not exist
    println!("Report2 errors: {:?}", report2.errors);
    assert!(!report2.errors.is_empty());

    // What happened to the dangling link in target_skills_dir?
    let link_still_present = engine.is_link(&link_path);
    assert!(
        !link_still_present,
        "Dangling link must be explicitly removed when library target is deleted!"
    );
}

#[test]
fn test_challenge_user_regular_file_collision() {
    let lib_dir = library_skills_dir();
    let temp_workspace = TempDir::new().unwrap();
    let target_skills_dir = temp_workspace.path().join(".gemini").join("skills");
    fs::create_dir_all(&target_skills_dir).unwrap();

    // User creates a REGULAR FILE with the name of a skill (e.g. "agent-preset-conductor")
    let colliding_file = target_skills_dir.join("agent-preset-conductor");
    fs::write(&colliding_file, "USER AUTHORED REGULAR FILE").unwrap();

    let desired = vec!["agent-preset-conductor".to_string()];
    let report = sync_skill_list(&lib_dir, &target_skills_dir, &desired).unwrap();

    // The user regular file MUST NOT be deleted or overwritten
    assert!(
        colliding_file.exists(),
        "User regular file must NOT be deleted by symlink engine!"
    );
    assert_eq!(
        fs::read_to_string(&colliding_file).unwrap(),
        "USER AUTHORED REGULAR FILE"
    );

    // Verify report reflects the failure/rejection to overwrite
    assert_eq!(report.added.len(), 0);
    assert!(!report.errors.is_empty());
    println!("User file collision errors: {:?}", report.errors);
}

#[test]
fn test_challenge_target_skills_dir_is_a_file() {
    let lib_dir = library_skills_dir();
    let temp_workspace = TempDir::new().unwrap();
    let target_skills_file = temp_workspace.path().join("skills_file");
    fs::write(&target_skills_file, "I am a file, not a directory").unwrap();

    // Try to sync into a path that is a regular file
    let desired = vec!["agent-preset-conductor".to_string()];
    let res = sync_skill_list(&lib_dir, &target_skills_file, &desired);

    assert!(res.is_err(), "Must return Err when target_skills_dir is a file");
    match res {
        Err(CoreError::Symlink(SymlinkError::NotADirectory(p))) => {
            assert_eq!(p, target_skills_file);
        }
        other => {
            panic!("Expected CoreError::Symlink(NotADirectory), got: {:?}", other);
        }
    }
}

