#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct CodemapTaxonomy {
    pub version: u16,
    pub metadata: Option<TaxonomyMetadata>,
    pub anchor_format: AnchorFormat,
    pub priorities: BTreeMap<String, PriorityDef>,
    pub layers: BTreeMap<String, LayerDef>,
    pub domains: BTreeMap<String, DomainDef>,
    pub roles: BTreeMap<String, RoleDef>,
    pub scoring: Option<ScoringDef>,
    pub validation: Option<ValidationDef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TaxonomyMetadata {
    pub name: String,
    pub purpose: String,
    pub owner: String,
    #[serde(default)]
    pub generated_files: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnchorFormat {
    pub marker: String,
    #[serde(default)]
    pub required_fields: Vec<String>,
    #[serde(default)]
    pub optional_fields: Vec<String>,
    pub default_priority: Option<String>,
    pub default_status: Option<String>,
    pub generated_anchor_prefix: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PriorityDef {
    pub label: String,
    pub description: String,
    pub score: i32,
    #[serde(default)]
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LayerDef {
    pub label: String,
    #[serde(default)]
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DomainDef {
    pub label: String,
    pub layer: String,
    pub description: String,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub preferred_roles: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoleDef {
    pub description: String,
    #[serde(default)]
    pub score: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScoringDef {
    #[serde(default)]
    pub anchor_priority: BTreeMap<String, i32>,
    #[serde(default)]
    pub role_bonus: BTreeMap<String, i32>,
    pub match_bonus: Option<BTreeMap<String, i32>>,
    pub penalties: Option<BTreeMap<String, i32>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ValidationDef {
    pub require_unique_anchor: Option<bool>,
    pub require_known_domain: Option<bool>,
    pub require_known_role: Option<bool>,
    pub require_known_priority: Option<bool>,
    pub warn_missing_priority: Option<bool>,
    pub warn_missing_layer: Option<bool>,
    pub warn_domain_path_mismatch: Option<bool>,
    pub warn_p0_without_manual_anchor: Option<bool>,
    pub warn_generated_anchor_with_p0: Option<bool>,
}

impl CodemapTaxonomy {
    pub fn load(root: &Path) -> Result<Self> {
        let text = fs::read_to_string(taxonomy_path(root))?;
        Ok(serde_yaml::from_str(&text)?)
    }

    pub fn try_load(root: &Path) -> Option<Self> {
        Self::load(root).ok()
    }

    pub fn default_priority(&self) -> String {
        self.anchor_format
            .default_priority
            .clone()
            .unwrap_or_else(|| "P2".to_string())
    }

    pub fn default_status(&self) -> String {
        self.anchor_format
            .default_status
            .clone()
            .unwrap_or_else(|| "stable".to_string())
    }

    pub fn priority_score(&self, priority: &str) -> i32 {
        self.priorities
            .get(priority)
            .map(|value| value.score)
            .or_else(|| {
                self.scoring
                    .as_ref()
                    .and_then(|scoring| scoring.anchor_priority.get(priority).copied())
            })
            .unwrap_or(0)
    }

    pub fn role_score(&self, role: &str) -> i32 {
        self.roles
            .get(role)
            .map(|value| value.score)
            .or_else(|| {
                self.scoring
                    .as_ref()
                    .and_then(|scoring| scoring.role_bonus.get(role).copied())
            })
            .unwrap_or(0)
    }

    pub fn known_domain(&self, domain: &str) -> bool {
        self.domains.contains_key(domain)
    }

    pub fn known_role(&self, role: &str) -> bool {
        self.roles.contains_key(role)
    }

    pub fn known_priority(&self, priority: &str) -> bool {
        self.priorities.contains_key(priority)
    }
}

pub fn taxonomy_path(root: &Path) -> PathBuf {
    root.join(".amigo").join("codemap.taxonomy.yml")
}
