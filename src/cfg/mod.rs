//! Generic cfg layer: lossless reading and rewriting of the game's
//! configuration files. No knowledge of the "spawn" domain.
#![warn(missing_docs)]

mod document;
mod lexer;

pub use document::{DocError, Document, Line, NoValueOnLine, ValueEdit};
pub use lexer::{lex_line, parse_attrs, LineKind, Span};
