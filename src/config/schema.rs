//! Configuration structures as written in the TOML
//! (raw deserialization, before checking and compilation).

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::domain::FieldKind;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawConfig {
    pub project: ProjectConfig,
    pub files: Vec<FileSpec>,
    pub parser: ParserConfig,
    pub ranks: RawRanks,
    pub fields: Vec<FieldSpec>,
    #[serde(default)]
    pub pairs: Vec<RawPair>,
    #[serde(default)]
    pub batch: Vec<RawBatch>,
    pub categories: Vec<RawCategory>,
    #[serde(default)]
    pub rules: Vec<RawRule>,
}

/// Working folders, relative to the mod root.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    /// Folder of the user's presets (e.g. `presets`).
    pub presets_dir: String,
}

/// A loaded cfg file.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileSpec {
    /// Path relative to the mod root (`/` separator).
    pub path: String,
    /// Short name displayed.
    pub name: String,
    /// Role, referenced by the classification rules.
    pub role: String,
}

/// cfg format key names used by the entry extraction.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParserConfig {
    /// Key that opens an item entry (e.g. `ItemPrototypeSID`).
    pub item_key: String,
    /// Key that opens a delegation (e.g. `ItemGeneratorPrototypeSID`).
    pub delegation_key: String,
    /// Value meaning "empty" (e.g. `empty`).
    pub empty_value: String,
    /// Level-1 SID key (e.g. `SID`), renames a `[N]` block.
    pub block_sid_key: String,
    /// Rank keys (e.g. `PlayerRank`, `PlayerRankMask`).
    pub rank_keys: Vec<String>,
    /// Caliber key (e.g. `AmmoCaliber`).
    pub caliber_key: String,
    /// "Absent" caliber value (e.g. `EAmmoCaliber::None`).
    pub none_value: String,
    /// Consumables set key (e.g. `IsItemSet`).
    pub itemset_key: String,
    /// Scope name → section label.
    #[serde(default)]
    pub sections: BTreeMap<String, String>,
    /// Aliases of cfg keys (misspellings in the vanilla data):
    /// alias → canonical key (e.g. `weight` → `Weight`).
    #[serde(default)]
    pub aliases: BTreeMap<String, String>,
    /// File roles for which the level-1 block is itself the editable unit
    /// (no `ItemPrototypeSID`): each block opens an entry whose fields are
    /// the flat keys directly under the block (anomalies, …).
    #[serde(default)]
    pub block_entry_roles: Vec<String>,
    /// File roles for which a scope whose name is a configured rank opens
    /// an entry for the whole block, scoped to that rank (world artifact
    /// spawners: one `Newbie`/`Experienced`/… sub-block per rank).
    #[serde(default)]
    pub rank_scope_roles: Vec<String>,
    /// Name of the scope listing the items an entry stands for (world
    /// spawners: `ListOfArtifacts`); the first listed SID becomes the
    /// display icon hint of the block's rows (never exported).
    #[serde(default)]
    pub icon_list_scope: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawRanks {
    pub values: Vec<String>,
}

/// Type of a cfg value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValueType {
    /// Integer.
    Int,
    /// Float.
    Float,
}

/// Display unit of a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Display {
    /// Raw cfg text.
    #[default]
    Raw,
    /// Percentage (`0.25f` displayed as `25%`).
    Percent,
}

/// Carrier of a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldScope {
    /// Carried by the entry (item).
    #[default]
    Entry,
    /// Shared by all the entries of the group (caliber / set).
    Group,
}

/// Schema of an editable field.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldSpec {
    /// Type of the field.
    pub kind: FieldKind,
    /// Short label (column header).
    pub short: String,
    /// Long label (tooltip, errors).
    pub long: String,
    /// Value type (integer or float).
    #[serde(rename = "type")]
    pub value_type: ValueType,
    /// Minimum bound (`None`: none).
    #[serde(default)]
    pub min: Option<f64>,
    /// Maximum bound (`None`: none).
    #[serde(default)]
    pub max: Option<f64>,
    /// Display unit.
    #[serde(default)]
    pub display: Display,
    /// Carrier of the field (entry or group).
    #[serde(default)]
    pub scope: FieldScope,
}

impl FieldSpec {
    /// Displayed as a percentage.
    pub fn is_percent(&self) -> bool {
        self.display == Display::Percent
    }

    /// Group field (shared).
    pub fn is_group(&self) -> bool {
        self.scope == FieldScope::Group
    }

    /// Integer value.
    pub fn is_int(&self) -> bool {
        self.value_type == ValueType::Int
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawPair {
    pub min: FieldKind,
    pub max: FieldKind,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawBatch {
    pub label: String,
    pub fields: Vec<FieldKind>,
    pub inputs: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawCategory {
    pub id: String,
    pub label: String,
    pub short: String,
    /// Sidebar section (dropdown); defaults to "ITEM SPAWNS".
    #[serde(default = "default_section")]
    pub section: String,
    pub group: String,
    pub columns: Vec<FieldKind>,
}

fn default_section() -> String {
    "ITEM SPAWNS".into()
}

/// Handling of delegations by a classification rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DelegationMode {
    /// Delegations do not match the rule.
    #[default]
    Exclude,
    /// Delegations match the rule.
    Include,
    /// Only delegations match the rule.
    Only,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawMatcher {
    #[serde(default)]
    pub exact: Vec<String>,
    #[serde(default)]
    pub prefix: Vec<String>,
    #[serde(default)]
    pub suffix: Vec<String>,
    #[serde(default)]
    pub contains: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawRule {
    pub category: String,
    #[serde(default)]
    pub delegations: DelegationMode,
    #[serde(default)]
    pub file_roles: Vec<String>,
    #[serde(default)]
    pub exclude_file_roles: Vec<String>,
    #[serde(default)]
    pub block: Option<RawMatcher>,
    #[serde(default)]
    pub item: Option<RawMatcher>,
    #[serde(default)]
    pub has_fields: Vec<FieldKind>,
}
