//! Classification of an entry into its categories, according to the
//! declarative rules of the configuration.
//!
//! Default: the first matching rule wins (one category per entry). Files
//! flagged `multi_category` collect all the matching rules: one entry may
//! belong to several categories (weapon stats — one weapon shows in its
//! "fire", "aiming", "recoil"… rows).

use crate::config::{CategoryId, Config, FileSpec, RuleInput};

use super::entry::Entry;
use super::field::Field;

/// Categories of an entry (empty = outside the scope of the application).
pub fn classify(
    cfg: &Config,
    file: &FileSpec,
    block_name: &str,
    entry: &Entry,
    fields: &[Field],
) -> Vec<CategoryId> {
    let has_field = |k| entry.fields.iter().any(|f| fields[f.idx()].kind == k);
    let input = RuleInput {
        is_delegation: entry.is_delegation(),
        file_role: &file.role,
        block: block_name,
        item: &entry.sid,
        has_field: &has_field,
    };
    let mut out = Vec::new();
    for r in &cfg.rules {
        if r.matches(&input) && !out.contains(&r.category) {
            out.push(r.category);
            // Single-category files: the first matching rule wins.
            if !file.multi_category {
                break;
            }
        }
    }
    out
}
