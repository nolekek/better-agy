use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use serde::{Deserialize, Serialize};

use crate::error::{CoreError, SymlinkError};
use crate::preset::PresetManifest;

/// Statistics / report returned after a synchronization operation
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncStats {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub kept: Vec<String>,
    pub preserved_user_dirs: Vec<String>,
    pub errors: Vec<(String, String)>,
    pub duration_ms: u64,
}

pub type SyncReport = SyncStats;

/// Symlink and Junction projection engine for skills
#[derive(Debug, Clone)]
pub struct SymlinkEngine {
    pub library_dir: PathBuf,
    pub target_skills_dir: PathBuf,
}

/// Helper: Checks if a directory exists and has 0 entries
fn is_directory_empty(path: &Path) -> bool {
    fs::read_dir(path)
        .map(|mut it| it.next().is_none())
        .unwrap_or(false)
}

impl SymlinkEngine {
    pub fn new(library_dir: PathBuf, target_skills_dir: PathBuf) -> Self {
        Self {
            library_dir,
            target_skills_dir,
        }
    }

    /// Checks if the given path is a symlink or an NTFS Junction
    pub fn is_link(&self, path: &Path) -> bool {
        #[cfg(windows)]
        {
            if let Ok(is_junc) = junction::exists(path) {
                if is_junc {
                    return true;
                }
            }
        }

        if let Ok(meta) = fs::symlink_metadata(path) {
            meta.file_type().is_symlink()
        } else {
            false
        }
    }

    /// Reads the target path of a symlink or junction
    pub fn read_link_target(&self, path: &Path) -> Option<PathBuf> {
        #[cfg(windows)]
        {
            if let Ok(is_junc) = junction::exists(path) {
                if is_junc {
                    if let Ok(target) = junction::get_target(path) {
                        return Some(target);
                    }
                }
            }
        }

        fs::read_link(path).ok()
    }

    /// Creates a directory symlink or NTFS junction pointing to `target` at `link_path`.
    pub fn create_link(&self, target: &Path, link_path: &Path) -> Result<(), SymlinkError> {
        if !target.exists() {
            return Err(SymlinkError::PathNotFound(target.to_path_buf()));
        }
        if !target.is_dir() {
            return Err(SymlinkError::NotADirectory(target.to_path_buf()));
        }

        let abs_target = target.canonicalize().unwrap_or_else(|_| target.to_path_buf());

        if self.is_link(link_path) || link_path.exists() {
            self.remove_link(link_path)?;
        }

        #[cfg(windows)]
        {
            match junction::create(&abs_target, link_path) {
                Ok(_) => Ok(()),
                Err(j_err) => {
                    if let Err(s_err) = std::os::windows::fs::symlink_dir(&abs_target, link_path) {
                        return Err(SymlinkError::LinkCreationFailed {
                            target: abs_target,
                            link: link_path.to_path_buf(),
                            reason: format!(
                                "Junction failed ({}) and symlink_dir failed ({})",
                                j_err, s_err
                            ),
                        });
                    }
                    Ok(())
                }
            }
        }

        #[cfg(not(windows))]
        {
            std::os::unix::fs::symlink(&abs_target, link_path).map_err(|e| {
                SymlinkError::LinkCreationFailed {
                    target: abs_target,
                    link: link_path.to_path_buf(),
                    reason: e.to_string(),
                }
            })?;
            Ok(())
        }
    }

    /// Safely removes a symlink or junction WITHOUT deleting files inside the target directory.
    /// Strictly refuses to delete non-symlink user directories.
    pub fn remove_link(&self, link_path: &Path) -> Result<(), SymlinkError> {
        #[cfg(windows)]
        {
            if let Ok(true) = junction::exists(link_path) {
                junction::delete(link_path).map_err(|e| SymlinkError::LinkRemovalFailed {
                    path: link_path.to_path_buf(),
                    reason: e.to_string(),
                })?;
                // Once junction reparse tag is stripped, remove hollow directory container.
                // Retry briefly to handle transient handle contention from concurrent readers.
                let mut removed = false;
                for _ in 0..5 {
                    if fs::remove_dir(link_path).is_ok() || !link_path.exists() {
                        removed = true;
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
                if !removed && link_path.exists() && is_directory_empty(link_path) {
                    let _ = fs::remove_dir(link_path);
                }
                return Ok(());
            }

            if let Ok(meta) = fs::symlink_metadata(link_path) {
                if meta.file_type().is_symlink() {
                    if let Err(e1) = fs::remove_dir(link_path) {
                        if let Err(e2) = fs::remove_file(link_path) {
                            return Err(SymlinkError::LinkRemovalFailed {
                                path: link_path.to_path_buf(),
                                reason: format!("remove_dir failed: {}, remove_file failed: {}", e1, e2),
                            });
                        }
                    }
                    return Ok(());
                }
            }

            // If it is an empty hollow directory (0 entries, e.g. from an interrupted junction removal),
            // it contains 0 user files and can be safely removed.
            if link_path.is_dir()
                && is_directory_empty(link_path)
                && (fs::remove_dir(link_path).is_ok() || !link_path.exists())
            {
                return Ok(());
            }

            // If neither junction nor symlink, and does not exist, nothing to do
            if !link_path.exists() && fs::symlink_metadata(link_path).is_err() {
                return Ok(());
            }

            Err(SymlinkError::LinkRemovalFailed {
                path: link_path.to_path_buf(),
                reason: "Refusing to delete non-symlink / non-junction directory".to_string(),
            })
        }

        #[cfg(not(windows))]
        {
            if let Ok(meta) = fs::symlink_metadata(link_path) {
                if meta.file_type().is_symlink() {
                    fs::remove_file(link_path).map_err(|e| SymlinkError::LinkRemovalFailed {
                        path: link_path.to_path_buf(),
                        reason: e.to_string(),
                    })?;
                    return Ok(());
                }
            }

            if link_path.is_dir()
                && is_directory_empty(link_path)
                && (fs::remove_dir(link_path).is_ok() || !link_path.exists())
            {
                return Ok(());
            }

            if !link_path.exists() && fs::symlink_metadata(link_path).is_err() {
                return Ok(());
            }

            Err(SymlinkError::LinkRemovalFailed {
                path: link_path.to_path_buf(),
                reason: "Refusing to delete non-symlink directory".to_string(),
            })
        }
    }

    /// Synchronizes target skills directory with the list of desired skill IDs.
    pub fn sync_skills(&self, desired_skills: &[String]) -> Result<SyncStats, SymlinkError> {
        let start = Instant::now();
        let mut report = SyncStats::default();

        if !self.library_dir.exists() {
            return Err(SymlinkError::PathNotFound(self.library_dir.clone()));
        }
        if !self.library_dir.is_dir() {
            return Err(SymlinkError::NotADirectory(self.library_dir.clone()));
        }

        // VULN-04: target_skills_dir cannot be a regular file
        if self.target_skills_dir.is_file() {
            return Err(SymlinkError::NotADirectory(self.target_skills_dir.clone()));
        }

        let library_canonical = self.library_dir.canonicalize().map_err(SymlinkError::Io)?;

        if !self.target_skills_dir.exists() {
            fs::create_dir_all(&self.target_skills_dir).map_err(SymlinkError::Io)?;
        }

        let desired_set: HashSet<String> = desired_skills.iter().cloned().collect();

        // 1. Scan currently active entries in target_skills_dir
        let mut current_links = HashSet::new();
        let mut current_user_dirs = HashSet::new();

        if let Ok(entries) = fs::read_dir(&self.target_skills_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = entry.file_name().to_string_lossy().to_string();

                if self.is_link(&path) {
                    current_links.insert(file_name);
                } else if path.is_dir() {
                    let is_empty = is_directory_empty(&path);
                    if is_empty && desired_set.contains(&file_name) {
                        // VULN-01: Hollow leftover at a desired skill location!
                        // Clean it up immediately so it can be re-linked in Step 4.
                        let _ = fs::remove_dir(&path);
                    } else if !is_empty {
                        current_user_dirs.insert(file_name);
                    } else {
                        // Empty user-created directory not matching any desired skill
                        current_user_dirs.insert(file_name);
                    }
                }
            }
        }

        // 2. Remove obsolete symlinks that are not desired
        for current in &current_links {
            if !desired_set.contains(current) {
                let link_path = self.target_skills_dir.join(current);
                match self.remove_link(&link_path) {
                    Ok(_) => report.removed.push(current.clone()),
                    Err(e) => report.errors.push((current.clone(), e.to_string())),
                }
            }
        }

        // 3. Preserve user-created regular directories
        for user_dir in &current_user_dirs {
            report.preserved_user_dirs.push(user_dir.clone());
        }

        // 4. Create or retain desired skills
        for desired in desired_skills {
            // Guard against traversal attacks in skill names
            if desired.contains('/') || desired.contains('\\') || desired == ".." {
                report.errors.push((
                    desired.clone(),
                    format!("Path traversal attempt detected in skill name '{}'", desired),
                ));
                continue;
            }

            let target_skill_dir = self.library_dir.join(desired);
            let link_path = self.target_skills_dir.join(desired);

            // VULN-02: Clean up dangling link if target folder is missing
            if !target_skill_dir.exists() {
                if self.is_link(&link_path) {
                    let _ = self.remove_link(&link_path);
                }
                report.errors.push((
                    desired.clone(),
                    format!("Skill library folder does not exist: {}", target_skill_dir.display()),
                ));
                continue;
            }

            let target_canonical = match target_skill_dir.canonicalize() {
                Ok(c) => c,
                Err(e) => {
                    report.errors.push((desired.clone(), e.to_string()));
                    continue;
                }
            };

            // Security assertion: target must reside within library_dir
            if !target_canonical.starts_with(&library_canonical) {
                report.errors.push((
                    desired.clone(),
                    format!(
                        "Security violation: target '{}' escapes library directory '{}'",
                        target_canonical.display(),
                        library_canonical.display()
                    ),
                ));
                continue;
            }

            // If a regular user directory exists with this name, preserve it and do not overwrite!
            if current_user_dirs.contains(desired) {
                let is_empty = link_path.is_dir() && is_directory_empty(&link_path);
                if is_empty {
                    // VULN-01: Hollow shell leftover: clean it up so it can be re-linked
                    let _ = fs::remove_dir(&link_path);
                } else {
                    report.errors.push((
                        desired.clone(),
                        format!(
                            "Skipped: regular user directory exists at target location '{}'",
                            link_path.display()
                        ),
                    ));
                    continue;
                }
            }

            // Check if link already exists and points to the correct location
            if current_links.contains(desired) && self.is_link(&link_path) {
                if let Some(existing_target) = self.read_link_target(&link_path) {
                    let existing_canon = existing_target
                        .canonicalize()
                        .unwrap_or_else(|_| existing_target.clone());

                    if normalize_path(&target_canonical) == normalize_path(&existing_canon) {
                        report.kept.push(desired.clone());
                        continue;
                    }
                }
            }

            // VULN-01: If link_path is a hollow directory, clean it before creating link
            if link_path.is_dir() && !self.is_link(&link_path) && is_directory_empty(&link_path) {
                let _ = fs::remove_dir(&link_path);
            }

            // Create or recreate link
            match self.create_link(&target_skill_dir, &link_path) {
                Ok(_) => report.added.push(desired.clone()),
                Err(e) => report.errors.push((desired.clone(), e.to_string())),
            }
        }

        report.added.sort();
        report.removed.sort();
        report.kept.sort();
        report.preserved_user_dirs.sort();
        report.duration_ms = start.elapsed().as_millis() as u64;

        Ok(report)
    }

    /// Synchronizes target skills directory with a specific preset from a manifest
    pub fn sync_preset(
        &self,
        active_preset: &str,
        manifest: &PresetManifest,
    ) -> Result<SyncStats, CoreError> {
        let desired = manifest.resolve_preset(active_preset)?;
        self.sync_skills(&desired).map_err(CoreError::from)
    }
}

/// Normalizes path removing Windows UNC `\\?\` prefix if present
fn normalize_path(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        PathBuf::from(stripped)
    } else {
        path.to_path_buf()
    }
}

/// Synchronizes skills for a workspace given library path, target skills directory,
/// active preset name, and parsed PresetManifest.
pub fn sync_skills<P1: AsRef<Path>, P2: AsRef<Path>>(
    library_path: P1,
    target_skills_dir: P2,
    active_preset: &str,
    manifest: &PresetManifest,
) -> Result<SyncStats, CoreError> {
    let desired_skills = manifest.resolve_preset(active_preset)?;
    sync_skill_list(library_path, target_skills_dir, &desired_skills)
}

/// Synchronizes skills for a workspace given library path, target skills directory,
/// and a list of desired skill IDs.
pub fn sync_skill_list<P1: AsRef<Path>, P2: AsRef<Path>>(
    library_path: P1,
    target_skills_dir: P2,
    desired_skills: &[String],
) -> Result<SyncStats, CoreError> {
    let engine = SymlinkEngine::new(
        library_path.as_ref().to_path_buf(),
        target_skills_dir.as_ref().to_path_buf(),
    );
    engine.sync_skills(desired_skills).map_err(CoreError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        let p1 = Path::new(r"\\?\C:\foo\bar");
        assert_eq!(normalize_path(p1), PathBuf::from(r"C:\foo\bar"));

        let p2 = Path::new(r"C:\foo\bar");
        assert_eq!(normalize_path(p2), PathBuf::from(r"C:\foo\bar"));

        let p3 = Path::new("/opt/gemini/skills");
        assert_eq!(normalize_path(p3), PathBuf::from("/opt/gemini/skills"));
    }

    #[test]
    fn test_sync_stats_serde_roundtrip() {
        let stats = SyncStats {
            added: vec!["skill-1".to_string(), "skill-2".to_string()],
            removed: vec!["skill-old".to_string()],
            kept: vec!["meta-skill".to_string()],
            preserved_user_dirs: vec!["my-custom".to_string()],
            errors: vec![],
            duration_ms: 42,
        };

        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: SyncStats = serde_json::from_str(&json).unwrap();
        assert_eq!(stats, deserialized);
    }
}

