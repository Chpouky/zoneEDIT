//! Discovery of the mod root and loading of the configured files.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::cfg::Document;
use crate::config::Config;
use crate::domain::{extract, Fingerprint, SourceFile, Workspace};
use crate::error::{Error, Result};

/// Remonte depuis `start` jusqu'au dossier qui contient le premier fichier
/// configured (the root marker).
pub fn find_root(start: &Path, cfg: &Config) -> Option<PathBuf> {
    let marker = &cfg.files.first()?.path;
    start
        .ancestors()
        .find(|p| p.join(marker).is_file())
        .map(Path::to_path_buf)
}

/// Disposition du dossier de travail ZoneKit : chemin relatif de la racine
/// du mod (`Stalker2/Content/GameLite`, qui contient `GameData/`) depuis le
/// dossier ZoneKit.
const ZONEKIT_ROOT: &str = "Stalker2/Content/GameLite";

/// Resolves the root of the mod (folder containing `GameData/`) from the
/// folder chosen by the user.
///
/// Two layouts:
/// 1. the chosen folder (or one of its ancestors) directly contains the
///    marker (the user chose the root or `GameData`);
/// 2. ZoneKit: the chosen folder (or an ancestor) contains
///    `Stalker2/Content/GameLite/GameData/`.
pub fn resolve_root(start: &Path, cfg: &Config) -> Option<PathBuf> {
    let marker = &cfg.files.first()?.path;
    // Disposition 1.
    if let Some(root) = find_root(start, cfg) {
        return Some(root);
    }
    // Disposition 2 (ZoneKit).
    start
        .ancestors()
        .find(|p| p.join(ZONEKIT_ROOT).join(marker).is_file())
        .map(|p| p.join(ZONEKIT_ROOT))
}

/// Empreinte actuelle d'un fichier (`None` si illisible).
pub fn fingerprint(path: &Path) -> Option<Fingerprint> {
    let m = std::fs::metadata(path).ok()?;
    Some(Fingerprint {
        mtime: m.modified().ok()?,
        size: m.len(),
    })
}

/// Loads a root with its effective configuration (default + override).
pub fn load(root: &Path) -> Result<Workspace> {
    load_with(root, Config::for_root(root)?)
}

/// Loads a root with a given configuration. The files are read and
/// parsed in parallel (one thread per file).
pub fn load_with(root: &Path, config: Arc<Config>) -> Result<Workspace> {
    let parts = std::thread::scope(|s| {
        let handles: Vec<_> = config
            .files
            .iter()
            .enumerate()
            .map(|(idx, spec)| {
                let config = &config;
                s.spawn(move || {
                    let path = root.join(&spec.path);
                    let bytes = std::fs::read(&path).map_err(|source| Error::Read {
                        path: path.clone(),
                        source,
                    })?;
                    let fp = fingerprint(&path).ok_or_else(|| Error::Read {
                        path: path.clone(),
                        source: std::io::Error::other("metadata unavailable"),
                    })?;
                    let doc = Document::parse(&bytes).map_err(|_| Error::Utf8 {
                        name: spec.path.clone(),
                    })?;
                    let x = extract(config, idx, &doc);
                    Ok((
                        SourceFile {
                            spec: spec.clone(),
                            path,
                            doc,
                            fingerprint: fp,
                        },
                        x,
                    ))
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("thread de chargement"))
            .collect::<Result<Vec<_>>>()
    })?;
    Ok(Workspace::assemble(root.to_path_buf(), config, parts))
}

/// Names of the files modified (or deleted) on disk since the load.
pub fn stale_files(ws: &Workspace) -> Vec<String> {
    ws.files()
        .iter()
        .filter(|f| fingerprint(&f.path) != Some(f.fingerprint))
        .map(|f| f.name().to_string())
        .collect()
}

/// Neighboring projects (sibling or child folders containing the marker), sorted.
pub fn scan_projects(current: &Path, cfg: &Config) -> Vec<PathBuf> {
    let Some(marker) = cfg.files.first().map(|f| f.path.as_str()) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = Vec::new();
    for base in [current, current.parent().unwrap_or(current)] {
        let Ok(rd) = std::fs::read_dir(base) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() && p.join(marker).is_file() {
                out.push(p.canonicalize().unwrap_or(p));
            }
        }
    }
    if let Ok(canon) = current.canonicalize() {
        out.push(canon);
    }
    out.sort();
    out.dedup();
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    /// Creates a fake ZoneKit: `<tmp>/Stalker2/Content/GameLite/GameData/<marker>`.
    /// Renvoie la racine du mod correspondante.
    fn zonekit(tmp: &Path) -> PathBuf {
        let cfg = Config::default_arc();
        let root = tmp.join(ZONEKIT_ROOT);
        let marker = cfg.files.first().unwrap().path.clone();
        std::fs::create_dir_all(root.join(&marker).parent().unwrap()).unwrap();
        std::fs::write(root.join(marker), "").unwrap();
        root
    }

    #[test]
    fn resolve_root_zonekit_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = Config::default_arc();
        let root = zonekit(tmp.path());

        // We choose the ZoneKit folder itself.
        assert_eq!(
            resolve_root(tmp.path(), &cfg).as_deref(),
            Some(root.as_path())
        );

        // On choisit un sous-dossier du ZoneKit.
        let sub = tmp.path().join("Stalker2/Content");
        assert_eq!(resolve_root(&sub, &cfg).as_deref(), Some(root.as_path()));

        // On choisit `GameData` directement (disposition 1).
        let gamedata = root.join("GameData");
        assert_eq!(
            resolve_root(&gamedata, &cfg).as_deref(),
            Some(root.as_path())
        );
    }

    #[test]
    fn resolve_root_missing_returns_none() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = Config::default_arc();
        assert_eq!(resolve_root(tmp.path(), &cfg), None);
    }
}
