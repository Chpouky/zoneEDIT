//! Compiled classification rules (declarative patterns of the config).

use std::collections::HashSet;

use super::schema::{DelegationMode, RawMatcher};
use super::CategoryId;
use crate::domain::FieldKind;

/// Pattern on a string: true if one of the lists matches.
#[derive(Debug, Clone, Default)]
pub struct Matcher {
    exact: HashSet<String>,
    prefix: Vec<String>,
    suffix: Vec<String>,
    contains: Vec<String>,
}

impl Matcher {
    pub(super) fn compile(raw: RawMatcher) -> Self {
        Self {
            exact: raw.exact.into_iter().collect(),
            prefix: raw.prefix,
            suffix: raw.suffix,
            contains: raw.contains,
        }
    }

    /// The string matches one of the patterns.
    pub fn matches(&self, s: &str) -> bool {
        self.exact.contains(s)
            || self.prefix.iter().any(|p| s.starts_with(p.as_str()))
            || self.suffix.iter().any(|p| s.ends_with(p.as_str()))
            || self.contains.iter().any(|p| s.contains(p.as_str()))
    }
}

/// What a rule needs to know about an entry.
pub struct RuleInput<'a> {
    /// The entry is a delegation.
    pub is_delegation: bool,
    /// Role of the file (from the config).
    pub file_role: &'a str,
    /// Name of the level-1 block.
    pub block: &'a str,
    /// SID of the item (or target of the delegation).
    pub item: &'a str,
    /// Predicate: does the entry carry a field of this type?
    pub has_field: &'a dyn Fn(FieldKind) -> bool,
}

/// Compiled rule: all the present conditions must be true.
#[derive(Debug, Clone)]
pub struct Rule {
    /// Category assigned when the rule matches.
    pub category: CategoryId,
    pub(super) delegations: DelegationMode,
    pub(super) file_roles: Vec<String>,
    pub(super) exclude_file_roles: Vec<String>,
    pub(super) block: Option<Matcher>,
    pub(super) exclude_block: Option<Matcher>,
    pub(super) item: Option<Matcher>,
    pub(super) has_fields: Vec<FieldKind>,
}

impl Rule {
    /// The rule matches the entry.
    pub fn matches(&self, e: &RuleInput<'_>) -> bool {
        let deleg_ok = match self.delegations {
            DelegationMode::Exclude => !e.is_delegation,
            DelegationMode::Include => true,
            DelegationMode::Only => e.is_delegation,
        };
        deleg_ok
            && (self.file_roles.is_empty() || self.file_roles.iter().any(|r| r == e.file_role))
            && !self.exclude_file_roles.iter().any(|r| r == e.file_role)
            && self.block.as_ref().is_none_or(|m| m.matches(e.block))
            && self
                .exclude_block
                .as_ref()
                .is_none_or(|m| !m.matches(e.block))
            && self.item.as_ref().is_none_or(|m| m.matches(e.item))
            && (self.has_fields.is_empty() || self.has_fields.iter().any(|k| (e.has_field)(*k)))
    }
}
