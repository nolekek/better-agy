use std::path::{Path, PathBuf};

pub fn resolve_presets_file(override_path: Option<&Path>) -> PathBuf {
    if let Some(p) = override_path {
        return p.to_path_buf();
    }
    let candidates = [
        "data/skills-library/presets.yaml",
        "/opt/gemini/skills-library/presets.yaml",
        "/opt/gemini/config/presets.yaml",
        "../../data/skills-library/presets.yaml",
    ];
    for c in &candidates {
        if Path::new(c).exists() {
            return PathBuf::from(c);
        }
    }
    PathBuf::from("data/skills-library/presets.yaml")
}

pub fn resolve_library_dir(override_path: Option<&Path>) -> PathBuf {
    if let Some(p) = override_path {
        return p.to_path_buf();
    }
    let candidates = [
        "data/skills-library",
        "/opt/gemini/skills-library",
        "../../data/skills-library",
    ];
    for c in &candidates {
        if Path::new(c).exists() {
            return PathBuf::from(c);
        }
    }
    PathBuf::from("data/skills-library")
}

pub fn run_preset_list(presets_file: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    let manifest_path = resolve_presets_file(presets_file);
    let manifest = agy_core::preset::load_presets(&manifest_path)?;

    println!("========================================================");
    println!(" Antigravity Presets Library");
    println!(" Source: {}", manifest_path.display());
    println!("========================================================");
    println!("Mandatory Meta-Skills (ALWAYS_ACTIVE):");
    for skill in &manifest.always_active {
        println!("  - {}", skill);
    }
    println!("\nAvailable Domain Presets (Total: {}):", manifest.presets.len());
    let mut names = manifest.presets.keys().collect::<Vec<_>>();
    names.sort();
    for name in names {
        let skills = manifest.resolve_preset(name)?;
        println!("  * {:<20} ({} skills)", name, skills.len());
    }
    println!("========================================================");
    Ok(())
}

pub fn run_preset_switch(
    preset_name: &str,
    workspace: &Path,
    library_dir: Option<&Path>,
    presets_file: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let manifest_path = resolve_presets_file(presets_file);
    let lib_path = resolve_library_dir(library_dir);
    let manifest = agy_core::preset::load_presets(&manifest_path)?;

    let target_skills_dir = workspace.join(".gemini").join("skills");

    println!("[*] Switching preset to '{}' in workspace '{}'...", preset_name, workspace.display());
    let stats = agy_core::symlink::sync_skills(&lib_path, &target_skills_dir, preset_name, &manifest)?;

    println!("[+] Successfully projected preset '{}':", preset_name);
    println!("    - Added symlinks: {}", stats.added.len());
    println!("    - Kept existing: {}", stats.kept.len());
    println!("    - Removed stale: {}", stats.removed.len());
    println!("    - Preserved user dirs: {}", stats.preserved_user_dirs.len());
    println!("    - Duration: {} ms", stats.duration_ms);
    Ok(())
}
