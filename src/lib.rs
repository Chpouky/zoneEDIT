//! zoneEDIT: desktop editor (egui) of the S.T.A.L.K.E.R. 2 spawn values.
//!
//! Layers:
//! - [`config`]: description of the game data (embedded TOML + override);
//! - [`cfg`]: lossless reading / rewriting of the `.cfg` files (generic);
//! - [`domain`]: entries, fields, classification, editing, validation;
//! - [`io`]: loading, export, presets;
//! - `ui`: egui interface.
//!
//! The original files are never written: the modifications live
//! in memory and the export writes copies in a mirror tree
//! (`<root>/exports/GameData/...`), ready to be copied into the mod folder.

pub mod cfg;
pub mod config;
pub mod domain;
pub mod error;
pub mod io;
pub mod ui;
pub mod ui_kit;
