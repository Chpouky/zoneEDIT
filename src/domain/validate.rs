//! Checking of the user's modifications. The original values of the game
//! are never checked (they have their own quirks, e.g.
//! `MinCount = 25 > MaxCount = 15` sur A762NATOD).

use std::fmt;

use super::field::FieldId;
use super::value;
use super::workspace::Workspace;

/// A validation error, localized.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    /// Name of the file concerned.
    pub file: String,
    /// 1-based line.
    pub line: usize,
    /// Faulty field.
    pub field: FieldId,
    /// Error message (EN, displayed as-is).
    pub message: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} — line {} : {}", self.file, self.line, self.message)
    }
}

impl Workspace {
    /// Validation errors of the modified fields, sorted by (file, line).
    ///
    /// - value not compliant with the schema of the field;
    /// - inconsistent (min, max) pair, only if the user modified
    ///   one of the two fields.
    pub fn validation_errors(&self) -> Vec<ValidationError> {
        let mut errs = Vec::new();
        for id in self.dirty_fields() {
            let f = self.field(id);
            if let Err(e) = value::check(self.spec(f.kind), f.value()) {
                errs.push(ValidationError {
                    file: self.files[f.file].name().to_string(),
                    line: f.line,
                    field: id,
                    message: e.to_string(),
                });
            }
        }
        for e in &self.entries {
            for (min_id, max_id) in self.count_pairs(e) {
                let (fmin, fmax) = (self.field(min_id), self.field(max_id));
                if !(fmin.is_dirty() || fmax.is_dirty()) {
                    continue;
                }
                if let (Some(min), Some(max)) = (fmin.num(), fmax.num()) {
                    if min > max {
                        errs.push(ValidationError {
                            file: self.files[e.file].name().to_string(),
                            line: self.entry_line(e),
                            field: min_id,
                            message: format!(
                                "{} (block {}) : {} ({min}) > {} ({max})",
                                e.display_name(),
                                self.block(e.block).name,
                                self.spec(fmin.kind).short,
                                self.spec(fmax.kind).short,
                            ),
                        });
                    }
                }
            }
        }
        errs.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
        errs
    }

    /// Number of modified fields with an invalid value (without the pairs).
    pub fn invalid_count(&self) -> usize {
        self.fields.iter().filter(|f| !f.is_valid()).count()
    }
}
