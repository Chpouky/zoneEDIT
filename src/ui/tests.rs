//! Headless UI tests (egui without GPU), split by theme:
//! - `common`: shared helpers (workspace, painted frames, clicks);
//! - `render`: painting / layout (topbar, table, ranks, spawn columns);
//! - `selection`: selection by checkbox, batch actions, errors panel;
//! - `behaviour`: reset, min/max validation, collapsed blocks, spawn chances;
//! - `menus_batch`: presets menu, batch modal "Set Min/Max Spawn %".
pub(crate) mod behaviour;
pub(crate) mod common;
pub(crate) mod menus_batch;
pub(crate) mod render;
pub(crate) mod selection;
