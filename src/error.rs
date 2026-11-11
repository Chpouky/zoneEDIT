//! Errors of the application (messages intended for the user, in English).

use std::path::PathBuf;

use crate::domain::ValidationError;

/// Standard result of zoneEDIT.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// All the errors bubbled up by the `config`, `cfg`, `domain`, `io` layers.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Cannot read {}: {source}", path.display())]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Cannot write {}: {source}", path.display())]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("{name}: invalid UTF-8 — the file contains non-UTF-8 bytes")]
    Utf8 { name: String },

    #[error("{name}: modified during the session — reload the files")]
    Stale { name: String },

    #[error("{name}: line {line} has no value to replace")]
    NoValueOnLine { name: String, line: usize },

    #[error("{name}: the field on line {line} is not inside a block — cannot generate a patch")]
    NoBlock { name: String, line: usize },

    #[error("Export cancelled — {} validation error(s):\n{}", .0.len(), join_lines(.0))]
    Validation(Vec<ValidationError>),

    #[error("Invalid configuration: {0}")]
    Config(String),

    #[error("Preset: {0}")]
    Preset(String),

    #[error("{marker} not found under {}", start.display())]
    RootNotFound { marker: String, start: PathBuf },
}

fn join_lines(errs: &[ValidationError]) -> String {
    errs.iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}
