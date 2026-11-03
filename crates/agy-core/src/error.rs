use std::path::PathBuf;
use thiserror::Error;

/// High-level core error type for `agy-core`
#[derive(Error, Debug)]
pub enum CoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Preset '{0}' not found in manifest")]
    PresetNotFound(String),

    #[error("Manifest missing required domain preset: '{0}'")]
    MissingDomainPreset(String),

    #[error("Manifest missing mandatory ALWAYS_ACTIVE meta-skill: '{0}'")]
    MissingAlwaysActiveMetaSkill(String),

    #[error("Invalid preset count: expected {expected}, found {found}")]
    InvalidPresetCount { expected: usize, found: usize },

    #[error("Disallowed legacy or unknown preset '{0}' (e.g. 'all' is permanently retired)")]
    DisallowedPreset(String),

    #[error("Invalid skill identifier '{0}': must conform to standard identifier regex '^[a-zA-Z0-9_-]+$'")]
    InvalidSkillIdentifier(String),

    #[error("Security violation: {0}")]
    SecurityViolation(String),

    #[error("Symlink engine error: {0}")]
    Symlink(#[from] SymlinkError),
}

/// Specialized error type for symlink projection engine operations
#[derive(Error, Debug)]
pub enum SymlinkError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Path '{0}' does not exist")]
    PathNotFound(PathBuf),

    #[error("Target '{0}' is not a directory")]
    NotADirectory(PathBuf),

    #[error("Security violation: target path '{target}' escapes allowed library '{library}'")]
    PathTraversal {
        target: PathBuf,
        library: PathBuf,
    },

    #[error("Failed to create link from '{target}' to '{link}': {reason}")]
    LinkCreationFailed {
        target: PathBuf,
        link: PathBuf,
        reason: String,
    },

    #[error("Failed to remove link at '{path}': {reason}")]
    LinkRemovalFailed {
        path: PathBuf,
        reason: String,
    },
}
