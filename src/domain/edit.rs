//! UNIQUE write point for values: user input (normalization,
//! %, validation, min ≤ max rule), batch actions, reset,
//! undo / redo. Each public operation = one history step.

use std::collections::BTreeSet;

use super::entry::EntryId;
use super::field::{FieldId, FieldKind};
use super::value::{self, ValueError};
use super::workspace::Workspace;

/// Maximum number of undo steps kept.
const HISTORY_LIMIT: usize = 200;

/// Modification of a field (current before / after values).
#[derive(Debug, Clone)]
struct Change {
    id: FieldId,
    before: Option<Box<str>>,
    after: Option<Box<str>>,
}

/// Undo history.
#[derive(Debug, Default)]
pub(crate) struct History {
    undo: Vec<Vec<Change>>,
    redo: Vec<Vec<Change>>,
}

/// Rejection of an input.
#[derive(Debug, Clone, PartialEq)]
pub enum EditError {
    /// Value rejected by the schema of the field.
    Invalid(ValueError),
    /// The (min, max) pair would become inconsistent.
    Pair {
        /// Short label of the min field.
        min_label: String,
        /// Short label of the max field.
        max_label: String,
        /// Proposed min value.
        min: f64,
        /// Effective max value.
        max: f64,
    },
}

impl std::fmt::Display for EditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(e) => write!(f, "{e}"),
            Self::Pair {
                min_label,
                max_label,
                min,
                max,
            } => write!(
                f,
                "{min_label} ({min}) > {max_label} ({max}) : value rejected (lower {min_label} or raise {max_label})."
            ),
        }
    }
}

/// Summary of a batch action.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct BatchReport {
    /// Fields effectively written.
    pub applied: usize,
    /// Rejected entries (invalid value or inconsistent pair).
    pub rejected: usize,
    /// First reason of rejection.
    pub first_error: Option<String>,
}

impl Workspace {
    // -----------------------------------------------------------------------
    // Primitives
    // -----------------------------------------------------------------------

    /// Applies a batch of changes as a single history step.
    fn apply(&mut self, changes: Vec<(FieldId, Option<Box<str>>)>) -> usize {
        let mut step = Vec::new();
        for (id, after) in changes {
            let before = self.fields[id.idx()].current().map(Into::into);
            let n = self.write(id, after);
            if n.as_deref() != before.as_deref() {
                step.push(Change {
                    id,
                    before,
                    after: n,
                });
            }
        }
        let n = step.len();
        if n > 0 {
            self.history.undo.push(step);
            if self.history.undo.len() > HISTORY_LIMIT {
                self.history.undo.remove(0);
            }
            self.history.redo.clear();
        }
        n
    }

    /// Writes a current value (normalized), keeps the counters up to date and
    /// returns the value effectively stored.
    fn write(&mut self, id: FieldId, value: Option<Box<str>>) -> Option<Box<str>> {
        let kind = self.fields[id.idx()].kind;
        let valid = value
            .as_deref()
            .is_none_or(|v| value::check(self.config.spec(kind), v).is_ok());
        let f = &mut self.fields[id.idx()];
        let was = f.is_dirty();
        f.set_current(value, valid);
        match (was, f.is_dirty()) {
            (false, true) => self.dirty += 1,
            (true, false) => self.dirty -= 1,
            _ => {}
        }
        self.revision += 1;
        f.current().map(Into::into)
    }

    /// Writes a cfg text as-is (uncontrolled) — presets, tests.
    pub fn set_text(&mut self, id: FieldId, text: &str) {
        self.apply(vec![(id, Some(text.into()))]);
    }

    /// Writes several cfg texts as-is, in one step.
    pub fn set_texts(&mut self, values: &[(FieldId, String)]) -> usize {
        self.apply(
            values
                .iter()
                .map(|(id, v)| (*id, Some(v.as_str().into())))
                .collect(),
        )
    }

    /// Replaces ALL the modifications with `values` (cfg texts written
    /// as-is), in one step: application of a preset.
    pub fn replace_all(&mut self, values: &[(FieldId, String)]) -> usize {
        let targets: BTreeSet<FieldId> = values.iter().map(|(id, _)| *id).collect();
        let mut changes: Vec<(FieldId, Option<Box<str>>)> = self
            .dirty_fields()
            .filter(|id| !targets.contains(id))
            .map(|id| (id, None))
            .collect();
        changes.extend(values.iter().map(|(id, v)| (*id, Some(v.as_str().into()))));
        self.apply(changes)
    }

    /// Restores fields to their original value (one step).
    pub fn reset(&mut self, ids: impl IntoIterator<Item = FieldId>) -> usize {
        self.apply(ids.into_iter().map(|id| (id, None)).collect())
    }

    /// Undoes all the modifications (one step).
    pub fn reset_all(&mut self) -> usize {
        let ids: Vec<FieldId> = self.dirty_fields().collect();
        self.reset(ids)
    }

    /// Restores all the fields (own + group) of the given entries.
    pub fn reset_entries(&mut self, entries: impl IntoIterator<Item = EntryId>) -> usize {
        let ids: BTreeSet<FieldId> = entries
            .into_iter()
            .flat_map(|e| self.entries[e.idx()].all_fields().collect::<Vec<_>>())
            .collect();
        self.reset(ids)
    }

    /// An undo step is available.
    pub fn can_undo(&self) -> bool {
        !self.history.undo.is_empty()
    }

    /// A redo step is available.
    pub fn can_redo(&self) -> bool {
        !self.history.redo.is_empty()
    }

    /// Undoes the last step; returns the number of fields touched.
    pub fn undo(&mut self) -> usize {
        let Some(step) = self.history.undo.pop() else {
            return 0;
        };
        for c in step.iter().rev() {
            self.write(c.id, c.before.clone());
        }
        let n = step.len();
        self.history.redo.push(step);
        n
    }

    /// Redoes the last undone step.
    pub fn redo(&mut self) -> usize {
        let Some(step) = self.history.redo.pop() else {
            return 0;
        };
        for c in &step {
            self.write(c.id, c.after.clone());
        }
        let n = step.len();
        self.history.undo.push(step);
        n
    }

    // -----------------------------------------------------------------------
    // Saisie utilisateur
    // -----------------------------------------------------------------------

    /// Input of a cell (display text: % for spawn chances).
    /// Refuse toute valeur invalide et toute paire min > max.
    pub fn commit_input(&mut self, id: FieldId, input: &str) -> Result<(), EditError> {
        let kind = self.fields[id.idx()].kind;
        let spec = self.config.spec(kind);
        let text = value::input_to_cfg(spec, input).map_err(EditError::Invalid)?;
        let new_v = value::parse_value(&text).unwrap_or(0.0);
        if let Some(owner) = self.owner(id) {
            self.check_pairs(owner, &[(kind, new_v)])?;
        }
        self.apply(vec![(id, Some(text.into()))]);
        Ok(())
    }

    /// Checks that applying `new` (type → value) leaves the pairs of
    /// the entry consistent.
    fn check_pairs(&self, e: EntryId, new: &[(FieldKind, f64)]) -> Result<(), EditError> {
        let e = &self.entries[e.idx()];
        for (min_id, max_id) in self.count_pairs(e) {
            let (kmin, kmax) = (self.field(min_id).kind, self.field(max_id).kind);
            if !new.iter().any(|(k, _)| *k == kmin || *k == kmax) {
                continue;
            }
            let get = |id: FieldId, k: FieldKind| {
                new.iter()
                    .find(|(nk, _)| *nk == k)
                    .map(|(_, v)| *v)
                    .or_else(|| self.field(id).num())
            };
            if let (Some(min), Some(max)) = (get(min_id, kmin), get(max_id, kmax)) {
                if min > max {
                    return Err(EditError::Pair {
                        min_label: self.config.spec(kmin).short.clone(),
                        max_label: self.config.spec(kmax).short.clone(),
                        min,
                        max,
                    });
                }
            }
        }
        Ok(())
    }

    /// Batch action: for each entry, writes the non-empty inputs
    /// (`(type, input)`) into its fields (own then group). An
    /// entry whose input is invalid or would violate min ≤ max is
    /// rejected in its entirety. A single history step.
    pub fn batch_input(
        &mut self,
        entries: &[EntryId],
        inputs: &[(FieldKind, &str)],
    ) -> BatchReport {
        let mut report = BatchReport::default();
        let mut parsed: Vec<(FieldKind, String, f64)> = Vec::new();
        for (kind, input) in inputs {
            if input.trim().is_empty() {
                continue;
            }
            match value::input_to_cfg(self.config.spec(*kind), input) {
                Ok(t) => {
                    let v = value::parse_value(&t).unwrap_or(0.0);
                    parsed.push((*kind, t, v));
                }
                Err(e) => {
                    report.rejected = entries.len();
                    report.first_error = Some(e.to_string());
                    return report;
                }
            }
        }
        let mut changes: Vec<(FieldId, Option<Box<str>>)> = Vec::new();
        let mut seen = BTreeSet::new();
        for &eid in entries {
            let new: Vec<(FieldKind, f64)> = parsed
                .iter()
                .filter(|(k, _, _)| self.entry_field(&self.entries[eid.idx()], *k).is_some())
                .map(|(k, _, v)| (*k, *v))
                .collect();
            if let Err(err) = self.check_pairs(eid, &new) {
                report.rejected += 1;
                report.first_error.get_or_insert_with(|| err.to_string());
                continue;
            }
            for (kind, text, _) in &parsed {
                if let Some(id) = self.entry_field(&self.entries[eid.idx()], *kind) {
                    if seen.insert(id) {
                        changes.push((id, Some(text.as_str().into())));
                    }
                }
            }
        }
        report.applied = changes.len();
        self.apply(changes);
        report
    }
}
