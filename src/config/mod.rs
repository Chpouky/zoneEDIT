//! zoneEDIT configuration: everything that describes the game data
//! (files, keys, fields, categories, classification rules…).
//!
//! The default configuration is embedded (`assets/config/default.toml`).
//! A `zoneedit.toml` file at the mod root overrides it: tables are merged
//! key by key, arrays are replaced wholesale.
#![warn(missing_docs)]

mod rules;
mod schema;

use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, OnceLock};

pub use rules::{Matcher, Rule, RuleInput};
pub use schema::{
    DelegationMode, FieldScope, FieldSpec, FileSpec, ParserConfig, ProjectConfig, ValueType,
};

use crate::domain::FieldKind;
use crate::error::{Error, Result};
use schema::RawConfig;

/// Text of the default configuration.
pub const DEFAULT_TOML: &str = include_str!("../../assets/config/default.toml");

/// Name of the override file, at the mod root.
pub const OVERRIDE_FILE: &str = "zoneedit.toml";

/// Identifier of a category (index in [`Config::categories`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct CategoryId(pub u16);

impl CategoryId {
    /// Index dans [`Config::categories`].
    pub fn idx(self) -> usize {
        self.0 as usize
    }
}

/// A category ("item type") of the sidebar.
#[derive(Debug, Clone)]
pub struct CategorySpec {
    /// Text identifier (referenced by rules and filters).
    pub id: String,
    /// Full label (sidebar).
    pub label: String,
    /// Short label.
    pub short: String,
    /// Sidebar section (dropdown label, e.g. "ITEM SPAWNS").
    pub section: String,
    /// Sidebar group (e.g. "Medical").
    pub group: String,
    /// Columns displayed for this category.
    pub columns: Vec<FieldKind>,
}

/// Un bouton d'action en lot.
#[derive(Debug, Clone)]
pub struct BatchSpec {
    /// Label of the button.
    pub label: String,
    /// Fields written by the action.
    pub fields: Vec<FieldKind>,
    /// Label of each input field (same length as `fields`).
    pub inputs: Vec<String>,
}

/// Compiled and checked configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Project parameters (export / presets folders).
    pub project: ProjectConfig,
    /// Loaded files, in display order.
    pub files: Vec<FileSpec>,
    /// Particular keys and values of the game's cfg format.
    pub parser: ParserConfig,
    /// Difficulty ranks, in order (bit `i` of the rank masks).
    pub ranks: Vec<String>,
    fields: Vec<Option<FieldSpec>>,
    /// (min, max) pairs checked together.
    pub pairs: Vec<(FieldKind, FieldKind)>,
    /// Actions en lot.
    pub batch: Vec<BatchSpec>,
    /// Categories, in the order of the config.
    pub categories: Vec<CategorySpec>,
    /// Classification rules, in order (the first matching one wins).
    pub rules: Vec<Rule>,
}

impl Config {
    /// Default configuration (parsed only once).
    pub fn default_arc() -> Arc<Config> {
        static DEFAULT: OnceLock<Arc<Config>> = OnceLock::new();
        DEFAULT
            .get_or_init(|| {
                Arc::new(Self::from_toml_str(DEFAULT_TOML).expect("default configuration"))
            })
            .clone()
    }

    /// Parses and compiles a complete configuration.
    pub fn from_toml_str(text: &str) -> Result<Config> {
        let raw: RawConfig = toml::from_str(text).map_err(|e| Error::Config(e.to_string()))?;
        Self::compile(raw)
    }

    /// Effective configuration of a mod root: default, overridden by
    /// `<root>/zoneedit.toml` s'il existe.
    pub fn for_root(root: &Path) -> Result<Arc<Config>> {
        let path = root.join(OVERRIDE_FILE);
        if !path.is_file() {
            return Ok(Self::default_arc());
        }
        let text = std::fs::read_to_string(&path).map_err(|source| Error::Read {
            path: path.clone(),
            source,
        })?;
        Self::with_override(&text).map(Arc::new)
    }

    /// Default + override (TOML text).
    pub fn with_override(override_text: &str) -> Result<Config> {
        let parse = |t: &str| {
            t.parse::<toml::Table>()
                .map_err(|e| Error::Config(e.to_string()))
        };
        let mut base = parse(DEFAULT_TOML)?;
        merge(&mut base, parse(override_text)?);
        let raw: RawConfig = toml::Value::Table(base)
            .try_into()
            .map_err(|e: toml::de::Error| Error::Config(e.to_string()))?;
        Self::compile(raw)
    }

    /// Schema of a field (`None` if the field is not configured).
    pub fn field(&self, kind: FieldKind) -> Option<&FieldSpec> {
        self.fields[kind.index()].as_ref()
    }

    /// Schema of a configured field. Fields extracted from the files always
    /// have a schema (only configured types are read).
    pub fn spec(&self, kind: FieldKind) -> &FieldSpec {
        self.field(kind)
            .unwrap_or_else(|| panic!("field {kind:?} not configured"))
    }

    /// Field type matching a cfg key (if configured). Keys are resolved
    /// through the configured aliases (vanilla misspellings, e.g. `weight`).
    pub fn field_kind_for_key(&self, key: &str) -> Option<FieldKind> {
        let canonical = self
            .parser
            .aliases
            .get(key)
            .map(String::as_str)
            .unwrap_or(key);
        FieldKind::from_key(canonical).filter(|k| self.field(*k).is_some())
    }

    /// Group field types, in declaration order of the types.
    pub fn group_kinds(&self) -> impl Iterator<Item = FieldKind> + '_ {
        FieldKind::ALL
            .into_iter()
            .filter(|k| self.field(*k).is_some_and(FieldSpec::is_group))
    }

    /// Category by identifier.
    pub fn category(&self, id: CategoryId) -> &CategorySpec {
        &self.categories[id.idx()]
    }

    /// Category by text identifier ("MedStash").
    pub fn category_id(&self, id: &str) -> Option<CategoryId> {
        self.categories
            .iter()
            .position(|c| c.id == id)
            .map(|i| CategoryId(i as u16))
    }

    /// All category identifiers.
    pub fn category_ids(&self) -> impl Iterator<Item = CategoryId> {
        (0..self.categories.len()).map(|i| CategoryId(i as u16))
    }

    /// Sections, in order of first appearance among the categories
    /// (items of the sidebar section dropdown).
    pub fn sections(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for c in &self.categories {
            if !out.iter().any(|s| s == &c.section) {
                out.push(c.section.clone());
            }
        }
        out
    }

    /// Ids of the categories belonging to a section (config order).
    pub fn categories_of_section(&self, section: &str) -> Vec<CategoryId> {
        self.category_ids()
            .filter(|id| self.category(*id).section == section)
            .collect()
    }

    /// Sidebar groups, in order of first appearance.
    pub fn category_groups(&self) -> Vec<(&str, Vec<CategoryId>)> {
        let mut out: Vec<(&str, Vec<CategoryId>)> = Vec::new();
        for id in self.category_ids() {
            let g = self.category(id).group.as_str();
            match out.iter_mut().find(|(name, _)| *name == g) {
                Some((_, v)) => v.push(id),
                None => out.push((g, vec![id])),
            }
        }
        out
    }

    /// Index d'un rang (`"Veteran"` → 2).
    pub fn rank_index(&self, name: &str) -> Option<usize> {
        self.ranks.iter().position(|r| r == name)
    }

    /// Mask containing all configured ranks.
    pub fn all_ranks_mask(&self) -> u32 {
        (1u32 << self.ranks.len()) - 1
    }

    // -----------------------------------------------------------------------

    fn compile(raw: RawConfig) -> Result<Config> {
        let err = |m: String| Err(Error::Config(m));
        if raw.files.is_empty() {
            return err("at least one [[files]] entry is required".into());
        }
        if raw.categories.is_empty() {
            return err("at least one [[categories]] entry is required".into());
        }
        if raw.ranks.values.len() > 31 {
            return err("at most 31 ranks are supported".into());
        }

        // Fields: one schema per type, no duplicate.
        let mut fields: Vec<Option<FieldSpec>> = vec![None; FieldKind::COUNT];
        for f in raw.fields {
            let slot = &mut fields[f.kind.index()];
            if slot.is_some() {
                return err(format!("field {:?} declared twice", f.kind));
            }
            if let (Some(a), Some(b)) = (f.min, f.max) {
                if a > b {
                    return err(format!("field {:?}: min > max", f.kind));
                }
            }
            *slot = Some(f);
        }
        let need_field = |k: FieldKind, ctx: &str| -> Result<()> {
            if fields[k.index()].is_none() {
                return Err(Error::Config(format!(
                    "{ctx}: field {k:?} is not declared in [[fields]]"
                )));
            }
            Ok(())
        };

        // Categories.
        let mut seen = HashSet::new();
        let mut categories = Vec::with_capacity(raw.categories.len());
        for c in raw.categories {
            if !seen.insert(c.id.clone()) {
                return err(format!("category {:?} declared twice", c.id));
            }
            for k in &c.columns {
                need_field(*k, &format!("category {:?}", c.id))?;
            }
            categories.push(CategorySpec {
                id: c.id,
                label: c.label,
                short: c.short,
                section: c.section,
                group: c.group,
                columns: c.columns,
            });
        }
        let cat_id = |id: &str, ctx: &str| -> Result<CategoryId> {
            categories
                .iter()
                .position(|c| c.id == id)
                .map(|i| CategoryId(i as u16))
                .ok_or_else(|| Error::Config(format!("{ctx}: unknown category {id:?}")))
        };

        // Rules.
        let mut rules = Vec::with_capacity(raw.rules.len());
        for (i, r) in raw.rules.into_iter().enumerate() {
            let ctx = format!("rule #{}", i + 1);
            for k in &r.has_fields {
                need_field(*k, &ctx)?;
            }
            rules.push(Rule {
                category: cat_id(&r.category, &ctx)?,
                delegations: r.delegations,
                file_roles: r.file_roles,
                exclude_file_roles: r.exclude_file_roles,
                block: r.block.map(Matcher::compile),
                item: r.item.map(Matcher::compile),
                has_fields: r.has_fields,
            });
        }

        // Rank-scope roles: the rank names come from the configured ranks.
        if !raw.parser.rank_scope_roles.is_empty() && raw.ranks.values.is_empty() {
            return err("rank_scope_roles requires at least one configured rank".into());
        }

        // Key aliases: the target must be a known field key.
        for (alias, target) in &raw.parser.aliases {
            if FieldKind::from_key(target).is_none() {
                return err(format!("parser alias {alias:?}: unknown key {target:?}"));
            }
        }

        // Paires et actions en lot.
        let mut pairs = Vec::with_capacity(raw.pairs.len());
        for p in raw.pairs {
            need_field(p.min, "pair")?;
            need_field(p.max, "pair")?;
            pairs.push((p.min, p.max));
        }
        let mut batch = Vec::with_capacity(raw.batch.len());
        for b in raw.batch {
            if b.fields.is_empty() || b.fields.len() != b.inputs.len() {
                return err(format!(
                    "batch {:?}: `fields` and `inputs` must be non-empty and of the same length",
                    b.label
                ));
            }
            for k in &b.fields {
                need_field(*k, &format!("batch {:?}", b.label))?;
            }
            batch.push(BatchSpec {
                label: b.label,
                fields: b.fields,
                inputs: b.inputs,
            });
        }

        Ok(Config {
            project: raw.project,
            files: raw.files,
            parser: raw.parser,
            ranks: raw.ranks.values,
            fields,
            pairs,
            batch,
            categories,
            rules,
        })
    }
}

/// Deep merge: tables merged recursively, the rest replaced.
fn merge(base: &mut toml::Table, over: toml::Table) {
    for (k, v) in over {
        match (base.get_mut(&k), v) {
            (Some(toml::Value::Table(b)), toml::Value::Table(o)) => merge(b, o),
            (_, v) => {
                base.insert(k, v);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let c = Config::default_arc();
        assert_eq!(c.files.len(), 6);
        assert_eq!(c.categories.len(), 18);
        assert_eq!(
            c.group_kinds().collect::<Vec<_>>(),
            vec![
                FieldKind::MainWeaponAmmoCount,
                FieldKind::MinSpawnChance,
                FieldKind::MaxSpawnChance
            ]
        );
        assert_eq!(c.category_groups()[0].0, "Medical");
        assert_eq!(c.all_ranks_mask(), 0b1111);
        // The vanilla `weight` misspelling is aliased to `Weight`.
        assert_eq!(c.field_kind_for_key("weight"), Some(FieldKind::Weight));
        assert_eq!(c.field_kind_for_key("Weight"), Some(FieldKind::Weight));
        assert_eq!(c.field_kind_for_key("WEIGHT"), None);
    }

    #[test]
    fn override_merges_tables_and_replaces_arrays() {
        let c = Config::with_override(
            "[project]\npresets_dir = \"mypresets\"\n\n[ranks]\nvalues = [\"Newbie\", \"Master\"]\n",
        )
        .unwrap();
        assert_eq!(
            c.project.presets_dir, "mypresets",
            "overridden key replaced"
        );
        assert_eq!(c.ranks, vec!["Newbie", "Master"]);
        assert_eq!(c.categories.len(), 18, "non-overridden array kept");
    }

    #[test]
    fn invalid_config_is_reported() {
        let e =
            Config::with_override("[[rules]]\ncategory = \"Nope\"\nitem = { exact = [\"X\"] }\n")
                .unwrap_err();
        assert!(e.to_string().contains("unknown category"), "{e}");
        let e = Config::with_override("[project]\nfoo = 1\n").unwrap_err();
        assert!(e.to_string().contains("unknown field"), "{e}");
        let e = Config::with_override("[parser.aliases]\nbogus = \"Nope\"\n").unwrap_err();
        assert!(e.to_string().contains("unknown key"), "{e}");
    }
}
