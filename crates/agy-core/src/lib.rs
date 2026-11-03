//! `agy-core`
//! 
//! Foundational library for Antigravity Linux Server:
//! - Presets manifest parsing and validation (`preset.rs`)
//! - Strict 16 domain presets and 5 mandatory ALWAYS_ACTIVE meta-skills guardrails
//! - High-performance atomic symlink projection engine (`symlink.rs`)
//! - Safe non-symlink user directory preservation and traversal security checks

pub mod error;
pub mod preset;
pub mod symlink;

// Re-export core types and functions
pub use error::{CoreError, SymlinkError};
pub use preset::{
    load_presets, load_presets_from_str, load_presets_unvalidated, parse_presets_unvalidated,
    HybridRule, Preset, PresetDefinition, PresetManifest, PresetsManifest, ResolvedPreset,
    ALWAYS_ACTIVE_META_SKILLS, REQUIRED_PRESETS,
};
pub use symlink::{
    sync_skill_list, sync_skills, SymlinkEngine, SyncReport, SyncStats,
};
