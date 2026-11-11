//! Persisted app state: the last folder opened, remembered between launches.
//!
//! The state lives in `$XDG_CONFIG_HOME/zoneedit/state.toml`
//! (fallback `~/.config/zoneedit/state.toml`):
//!
//! ```toml
//! last_root = "/path/to/STALKER2ZoneKit"
//! ```
//!
//! Everything here is silent by design: a missing, unreadable or corrupt
//! state file must never prevent the app from starting, and a failed save
//! must never surface in the UI.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Persisted app state.
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AppState {
    /// Folder last opened in the app (the path chosen in the OS picker).
    pub last_root: Option<PathBuf>,
}

/// State file: `$XDG_CONFIG_HOME/zoneedit/state.toml` or
/// `~/.config/zoneedit/state.toml` when `XDG_CONFIG_HOME` is unset.
pub fn state_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))?;
    Some(base.join("zoneedit/state.toml"))
}

/// Loads the persisted state. Missing or corrupt file → default.
pub fn load() -> AppState {
    state_path().map(|p| load_from(&p)).unwrap_or_default()
}

/// Saves the state to the state file (creates the parent directory).
/// Silent on error.
pub fn save(state: &AppState) {
    if let Some(path) = state_path() {
        let _ = save_to(&path, state);
    }
}

/// Remembers `dir` as the last opened folder.
pub fn save_last_root(dir: &Path) {
    save(&AppState {
        last_root: Some(dir.to_path_buf()),
    });
}

/// Loads the state from `path`. Missing or corrupt file → default.
fn load_from(path: &Path) -> AppState {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| toml::from_str(&raw).ok())
        .unwrap_or_default()
}

/// Saves the state to `path` (creates the parent directory).
fn save_to(path: &Path, state: &AppState) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let raw = toml::to_string_pretty(state).map_err(std::io::Error::other)?;
    std::fs::write(path, raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nested/state.toml");
        let state = AppState {
            last_root: Some(tmp.path().join("project")),
        };
        save_to(&path, &state).unwrap();
        let back = load_from(&path);
        assert_eq!(back.last_root, state.last_root);
    }

    #[test]
    fn missing_file_gives_default() {
        let tmp = tempfile::tempdir().unwrap();
        let state = load_from(&tmp.path().join("absent.toml"));
        assert!(state.last_root.is_none());
    }

    #[test]
    fn corrupt_file_gives_default() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("bad.toml");
        std::fs::write(&path, "this is not toml [[[").unwrap();
        let state = load_from(&path);
        assert!(state.last_root.is_none());
    }

    #[test]
    fn saves_last_root_shape() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("state.toml");
        save_to(
            &path,
            &AppState {
                last_root: Some(tmp.path().join("kit")),
            },
        )
        .unwrap();
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(raw.contains("last_root"));
    }
}
