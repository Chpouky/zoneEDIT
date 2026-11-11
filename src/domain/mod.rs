//! "Item spawn" domain: entries, editable fields, classification,
//! editing and validation. Built on top of the generic [`crate::cfg`] layer.
#![warn(missing_docs)]

mod classify;
mod edit;
mod entry;
mod extract;
mod field;
mod validate;
pub mod value;
mod workspace;

pub use classify::classify;
pub use edit::{BatchReport, EditError};
pub use entry::{Block, BlockId, Entry, EntryContext, EntryId, EntryKind};
pub use extract::{extract, Extracted};
pub use field::{Field, FieldId, FieldKind, FieldLoc};
pub use validate::ValidationError;
pub use workspace::{Fingerprint, Orphan, SourceFile, Workspace};
