//! Classification of an entry into a category, according to the
//! declarative rules of the configuration (first matching rule).

use crate::config::{CategoryId, Config, RuleInput};

use super::entry::Entry;
use super::field::Field;

/// Category of an entry; `None` = outside the scope of the application.
pub fn classify(
    cfg: &Config,
    file_role: &str,
    block_name: &str,
    entry: &Entry,
    fields: &[Field],
) -> Option<CategoryId> {
    let has_field = |k| entry.fields.iter().any(|f| fields[f.idx()].kind == k);
    let input = RuleInput {
        is_delegation: entry.is_delegation(),
        file_role,
        block: block_name,
        item: &entry.sid,
        has_field: &has_field,
    };
    cfg.rules
        .iter()
        .find(|r| r.matches(&input))
        .map(|r| r.category)
}
