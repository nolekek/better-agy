use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use serde::{Deserialize, Deserializer, Serialize};

use crate::error::CoreError;

/// The 16 canonical domain presets in Antigravity Linux Server
pub const REQUIRED_PRESETS: [&str; 16] = [
    "web",
    "backend",
    "devops",
    "appsec",
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
];

/// The 5 mandatory meta-skills that must ALWAYS be active
pub const ALWAYS_ACTIVE_META_SKILLS: [&str; 5] = [
    "agent-preset-conductor",
    "grill-me",
    "systematic-debugging",
    "verification-before-completion",
    "concise-planning",
];

fn default_version() -> String {
    "3.0".to_string()
}

/// Validates that a skill name conforms to standard identifier syntax: `^[a-zA-Z0-9_-]+$`
pub fn is_valid_skill_identifier(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Definition of an individual preset
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PresetDefinition {
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub skills: Vec<String>,
    pub adjacent_presets: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
}

impl PresetDefinition {
    pub fn new<S: Into<String>>(name: S, skills: Vec<String>) -> Self {
        Self {
            name: name.into(),
            icon: None,
            description: None,
            skills,
            adjacent_presets: None,
            tags: None,
        }
    }
}

impl std::ops::Deref for PresetDefinition {
    type Target = [String];
    fn deref(&self) -> &Self::Target {
        &self.skills
    }
}

/// Custom deserialization for PresetDefinition to accept both structured mapping
/// and simple list of skill strings.
impl<'de> Deserialize<'de> for PresetDefinition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct FullPreset {
            #[serde(default)]
            name: Option<String>,
            #[serde(default)]
            icon: Option<String>,
            #[serde(default)]
            description: Option<String>,
            #[serde(default)]
            skills: Vec<String>,
            #[serde(default)]
            adjacent_presets: Option<Vec<String>>,
            #[serde(default)]
            tags: Option<Vec<String>>,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum PresetHelper {
            Simple(Vec<String>),
            Full(FullPreset),
        }

        let helper = PresetHelper::deserialize(deserializer)?;
        match helper {
            PresetHelper::Full(f) => Ok(PresetDefinition {
                name: f.name.unwrap_or_default(),
                icon: f.icon,
                description: f.description,
                skills: f.skills,
                adjacent_presets: f.adjacent_presets,
                tags: f.tags,
            }),
            PresetHelper::Simple(skills) => Ok(PresetDefinition {
                name: String::new(),
                icon: None,
                description: None,
                skills,
                adjacent_presets: None,
                tags: None,
            }),
        }
    }
}

pub type Preset = PresetDefinition;

/// Hybrid composition rule for combining presets
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HybridRule {
    pub trigger: Vec<String>,
    pub compose: Vec<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Parsed `presets.yaml` manifest
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PresetManifest {
    #[serde(default = "default_version")]
    pub version: String,
    pub always_active: Vec<String>,
    pub presets: HashMap<String, PresetDefinition>,
    #[serde(default)]
    pub hybrid_rules: Vec<HybridRule>,
}

pub type PresetsManifest = PresetManifest;

/// Resolved preset payload for workspace projection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedPreset {
    pub base: String,
    pub overlays: Vec<String>,
    pub active_skills: Vec<String>,
    pub total_skills: usize,
}

impl PresetManifest {
    /// Validates that the manifest conforms to the Antigravity architecture invariants:
    /// 1. Exactly 16 domain presets.
    /// 2. All 5 ALWAYS_ACTIVE meta-skills are declared in `always_active`.
    /// 3. Disallowed legacy presets (such as 'all') are absent.
    pub fn validate(&self) -> Result<(), CoreError> {
        // 1. Validate mandatory ALWAYS_ACTIVE meta-skills
        for &required_meta in &ALWAYS_ACTIVE_META_SKILLS {
            if !self.always_active.iter().any(|s| s == required_meta) {
                return Err(CoreError::MissingAlwaysActiveMetaSkill(
                    required_meta.to_string(),
                ));
            }
        }

        // 2. Reject permanently retired 'all' preset
        if self.presets.contains_key("all") {
            return Err(CoreError::DisallowedPreset("all".to_string()));
        }

        // 3. Validate preset count
        if self.presets.len() != REQUIRED_PRESETS.len() {
            return Err(CoreError::InvalidPresetCount {
                expected: REQUIRED_PRESETS.len(),
                found: self.presets.len(),
            });
        }

        // 4. Validate each required domain preset exists
        for &req_preset in &REQUIRED_PRESETS {
            if !self.presets.contains_key(req_preset) {
                return Err(CoreError::MissingDomainPreset(req_preset.to_string()));
            }
        }

        // 5. Validate always_active skill identifiers conform to ^[a-zA-Z0-9_-]+$
        for skill in &self.always_active {
            if !is_valid_skill_identifier(skill) {
                return Err(CoreError::InvalidSkillIdentifier(skill.clone()));
            }
        }

        // 6. Validate all preset skill identifiers conform to ^[a-zA-Z0-9_-]+$
        for preset_def in self.presets.values() {
            for skill in &preset_def.skills {
                if !is_valid_skill_identifier(skill) {
                    return Err(CoreError::InvalidSkillIdentifier(skill.clone()));
                }
            }
        }

        Ok(())
    }

    /// Resolves active skills for a preset, mandatorily injecting all 5 ALWAYS_ACTIVE meta-skills.
    /// Returns a sorted and deduplicated vector of skill names.
    pub fn resolve_preset(&self, preset_name: &str) -> Result<Vec<String>, CoreError> {
        let preset = self.presets.get(preset_name).ok_or_else(|| {
            CoreError::PresetNotFound(preset_name.to_string())
        })?;

        let mut skills_set = BTreeSet::new();

        // Always inject mandatory meta-skills
        for meta in &self.always_active {
            skills_set.insert(meta.clone());
        }
        for &meta in &ALWAYS_ACTIVE_META_SKILLS {
            skills_set.insert(meta.to_string());
        }

        // Add domain skills from preset
        for skill in &preset.skills {
            skills_set.insert(skill.clone());
        }

        Ok(skills_set.into_iter().collect())
    }

    /// Resolves active skills combining base preset with overlay presets.
    /// Always injects all 5 ALWAYS_ACTIVE meta-skills.
    pub fn resolve_preset_with_overlays(
        &self,
        base_preset: &str,
        overlays: &[&str],
    ) -> Result<Vec<String>, CoreError> {
        let mut skills_set: BTreeSet<String> = self.resolve_preset(base_preset)?.into_iter().collect();

        for overlay in overlays {
            let overlay_def = self.presets.get(*overlay).ok_or_else(|| {
                CoreError::PresetNotFound(overlay.to_string())
            })?;
            for skill in &overlay_def.skills {
                skills_set.insert(skill.clone());
            }
        }

        Ok(skills_set.into_iter().collect())
    }

    /// Returns a map of preset_name -> resolved skill list (with ALWAYS_ACTIVE included)
    pub fn skills_map(&self) -> HashMap<String, Vec<String>> {
        let mut map = HashMap::new();
        for preset_name in self.presets.keys() {
            if let Ok(skills) = self.resolve_preset(preset_name) {
                map.insert(preset_name.clone(), skills);
            }
        }
        map
    }

    /// Returns sorted list of preset IDs
    pub fn preset_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.presets.keys().cloned().collect();
        names.sort();
        names
    }
}

/// Loads and validates `presets.yaml` from file path
pub fn load_presets<P: AsRef<Path>>(path: P) -> Result<PresetManifest, CoreError> {
    let content = std::fs::read_to_string(path)?;
    load_presets_from_str(&content)
}

/// Loads and validates `presets.yaml` from YAML string
pub fn load_presets_from_str(content: &str) -> Result<PresetManifest, CoreError> {
    let manifest: PresetManifest = serde_yaml::from_str(content)?;
    manifest.validate()?;
    Ok(manifest)
}

/// Loads `presets.yaml` from file without applying validation checks (for testing)
pub fn load_presets_unvalidated<P: AsRef<Path>>(path: P) -> Result<PresetManifest, CoreError> {
    let content = std::fs::read_to_string(path)?;
    parse_presets_unvalidated(&content)
}

/// Parses `presets.yaml` string without applying validation checks (for testing)
pub fn parse_presets_unvalidated(content: &str) -> Result<PresetManifest, CoreError> {
    let manifest: PresetManifest = serde_yaml::from_str(content)?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_definition_simple_list_deserialization() {
        let yaml = r#"
- skill-a
- skill-b
"#;
        let def: PresetDefinition = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(def.skills, vec!["skill-a", "skill-b"]);
        assert_eq!(&def[..], &["skill-a", "skill-b"]);
    }

    #[test]
    fn test_preset_definition_full_deserialization() {
        let yaml = r#"
name: "Test Preset"
icon: "🧪"
description: "A test preset"
skills:
  - test-skill-1
"#;
        let def: PresetDefinition = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(def.name, "Test Preset");
        assert_eq!(def.icon.as_deref(), Some("🧪"));
        assert_eq!(def.description.as_deref(), Some("A test preset"));
        assert_eq!(def.skills, vec!["test-skill-1"]);
    }

    #[test]
    fn test_preset_names_sorted() {
        let mut manifest = PresetManifest {
            version: "3.0".to_string(),
            always_active: vec![],
            presets: HashMap::new(),
            hybrid_rules: vec![],
        };
        manifest.presets.insert("zebra".to_string(), PresetDefinition::new("Zebra", vec![]));
        manifest.presets.insert("alpha".to_string(), PresetDefinition::new("Alpha", vec![]));
        manifest.presets.insert("beta".to_string(), PresetDefinition::new("Beta", vec![]));

        assert_eq!(manifest.preset_names(), vec!["alpha", "beta", "zebra"]);
    }
}

