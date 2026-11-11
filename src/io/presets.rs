//! User presets: a preset = the modified values of a session.
//!
//! Format v3 (TOML): each field is identified by its context (file
//! path, block, item, occurrence) and carries its original value,
//! checked at application (a game patch changing the original value
//! makes the field "dubious": it is ignored). The v1 (line number)
//! and v2 (`|` text) formats remain readable.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::domain::{EntryId, EntryKind, FieldId, FieldKind, FieldLoc, Workspace};
use crate::error::{Error, Result};

const EXT: &str = "preset";
const FORMAT: u32 = 3;
const V1_HEADER: &str = "# zoneEdit preset : ";
const V2_HEADER: &str = "# zoneEdit preset v2 : ";

/// Cible d'un champ de preset.
#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    /// By context (v2, v3).
    Context {
        /// Relative path of the file (e.g. `ItemGeneratorPrototypes.cfg`).
        file: String,
        /// Name of the level-1 block.
        block: String,
        /// Nature of the entry.
        kind: EntryKind,
        /// SID of the item or delegation.
        sid: String,
        /// Occurrence among the entries with the same (block, kind, SID).
        occurrence: u32,
    },
    /// By line number (v1, or group field without entry).
    Line {
        /// Relative path of the file.
        file: String,
        /// 1-based line in the file.
        line: usize,
    },
}

/// Un champ d'un preset.
#[derive(Debug, Clone, PartialEq)]
pub struct PresetField {
    /// Target of the field in the game.
    pub target: Target,
    /// Type of the field.
    pub kind: FieldKind,
    /// Value to write (cfg text).
    pub value: String,
    /// Expected original value; `None` = no check (v1).
    pub original: Option<String>,
}

/// A loaded preset.
#[derive(Debug, Clone)]
pub struct Preset {
    /// Nom du preset (telle la saisie de l'utilisateur).
    pub name: String,
    /// Version du format lu (1, 2 ou 3).
    pub version: u32,
    /// Path of the preset file.
    pub path: PathBuf,
    /// Champs du preset, dans l'ordre du fichier.
    pub fields: Vec<PresetField>,
}

/// Bilan d'application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyReport {
    /// Fields applied to the workspace.
    pub applied: usize,
    /// Fields ignored (not found or original value modified).
    pub skipped: usize,
}

// ---------------------------------------------------------------------------
// Format v3 (serialized)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PresetFile {
    format: u32,
    name: String,
    #[serde(default)]
    app_version: String,
    #[serde(default)]
    fields: Vec<Row>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    file: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    block: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    item: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    delegation: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    occurrence: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
    field: FieldKind,
    value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    original: Option<String>,
}

// ---------------------------------------------------------------------------
// Fichiers
// ---------------------------------------------------------------------------

/// Dossier des presets d'une racine.
pub fn presets_dir(root: &Path, cfg: &Config) -> PathBuf {
    root.join(&cfg.project.presets_dir)
}

/// File name derived from the name (lowercase, `_` for everything else,
/// runs of `_` reduced to one).
pub fn slug(name: &str) -> String {
    let mut s = String::new();
    for c in name.chars() {
        if c.is_alphanumeric() {
            s.extend(c.to_lowercase());
        } else if !s.ends_with('_') {
            s.push('_');
        }
    }
    s.trim_matches('_').to_string()
}

fn preset_path(root: &Path, cfg: &Config, name: &str) -> Result<PathBuf> {
    let s = slug(name);
    if s.is_empty() {
        return Err(Error::Preset("invalid preset name".into()));
    }
    Ok(presets_dir(root, cfg).join(format!("{s}.{EXT}")))
}

/// Saves the modified fields of the workspace into a new v3 preset.
/// Refuses to overwrite an existing preset (same slug).
pub fn save(ws: &Workspace, name: &str) -> Result<PathBuf> {
    let path = preset_path(&ws.root, ws.config(), name)?;
    if path.is_file() {
        return Err(Error::Preset(format!(
            "a preset with this name already exists ({})",
            path.file_name().unwrap_or_default().to_string_lossy()
        )));
    }
    let rows: Vec<Row> = ws
        .dirty_fields()
        .map(|id| {
            let f = ws.field(id);
            let file = ws.files()[f.file].spec.path.clone();
            let ctx = ws.owner(id).map(|e| ws.entry(e));
            Row {
                file,
                block: ctx.map(|e| ws.block(e.block).name.clone()),
                item: ctx.map(|e| e.sid.clone()),
                delegation: ctx.is_some_and(|e| e.is_delegation()),
                occurrence: ctx.map(|e| e.occurrence),
                line: ctx.is_none().then_some(f.line),
                field: f.kind,
                value: f.value().to_string(),
                original: Some(f.original().to_string()),
            }
        })
        .collect();
    let file = PresetFile {
        format: FORMAT,
        name: name.to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        fields: rows,
    };
    let body = toml::to_string(&file).map_err(|e| Error::Preset(e.to_string()))?;
    let text = format!("# zoneEDIT preset (format {FORMAT})\n{body}");
    let dir = presets_dir(&ws.root, ws.config());
    std::fs::create_dir_all(&dir).map_err(|source| Error::Write {
        path: dir.clone(),
        source,
    })?;
    super::export::write_atomic(&path, text.as_bytes()).map_err(|source| Error::Write {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

/// Supprime le preset de ce nom.
pub fn delete(root: &Path, cfg: &Config, name: &str) -> Result<()> {
    let path = preset_path(root, cfg, name)?;
    if !path.is_file() {
        return Err(Error::Preset(format!(
            "preset not found ({})",
            path.display()
        )));
    }
    std::fs::remove_file(&path).map_err(|source| Error::Write { path, source })
}

/// Loads all the presets, sorted by name. Unreadable files are
/// reported in the second list (they are not silently ignored).
pub fn load_all(root: &Path, cfg: &Config) -> (Vec<Preset>, Vec<String>) {
    let mut out = Vec::new();
    let mut errors = Vec::new();
    let Ok(rd) = std::fs::read_dir(presets_dir(root, cfg)) else {
        return (out, errors);
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) != Some(EXT) {
            continue;
        }
        let res = std::fs::read_to_string(&p)
            .map_err(|e| e.to_string())
            .and_then(|text| parse(&text, &p, cfg));
        match res {
            Ok(preset) => out.push(preset),
            Err(msg) => errors.push(format!(
                "{}: {msg}",
                p.file_name().unwrap_or_default().to_string_lossy()
            )),
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    (out, errors)
}

/// Lit un preset (v1, v2 ou v3).
fn parse(text: &str, path: &Path, cfg: &Config) -> std::result::Result<Preset, String> {
    let header = text.lines().next().unwrap_or_default();
    let file_path = |idx: &str| -> Option<String> {
        cfg.files
            .get(idx.parse::<usize>().ok()?)
            .map(|f| f.path.clone())
    };
    if let Some(name) = header.strip_prefix(V2_HEADER) {
        // file|block|item|occ|Kind = value|original
        let mut fields = Vec::new();
        for line in data_lines(text) {
            let parsed = (|| {
                let (head, tail) = line.split_once(" = ")?;
                let (value, original) = tail.split_once('|')?;
                let parts: Vec<&str> = head.split('|').collect();
                let [file, block, item, occ, kind] = parts.as_slice() else {
                    return None;
                };
                let (kind_e, sid) = match item.strip_prefix("→ ") {
                    Some(t) => (EntryKind::Delegation, t),
                    None => (EntryKind::Item, *item),
                };
                Some(PresetField {
                    target: Target::Context {
                        file: file_path(file)?,
                        block: (*block).to_string(),
                        kind: kind_e,
                        sid: sid.to_string(),
                        occurrence: occ.parse().ok()?,
                    },
                    kind: FieldKind::from_key(kind)?,
                    value: value.to_string(),
                    original: Some(original.to_string()),
                })
            })();
            fields.push(parsed.ok_or_else(|| format!("invalid v2 line {line:?}"))?);
        }
        return Ok(Preset {
            name: name.to_string(),
            version: 2,
            path: path.to_path_buf(),
            fields,
        });
    }
    if let Some(name) = header.strip_prefix(V1_HEADER) {
        // file:line:Kind = value
        let mut fields = Vec::new();
        for line in data_lines(text) {
            let parsed = (|| {
                let (head, value) = line.split_once(" = ")?;
                let mut it = head.splitn(3, ':');
                let (file, l, kind) = (it.next()?, it.next()?, it.next()?);
                Some(PresetField {
                    target: Target::Line {
                        file: file_path(file)?,
                        line: l.parse().ok()?,
                    },
                    kind: FieldKind::from_key(kind)?,
                    value: value.to_string(),
                    original: None,
                })
            })();
            fields.push(parsed.ok_or_else(|| format!("invalid v1 line {line:?}"))?);
        }
        return Ok(Preset {
            name: name.to_string(),
            version: 1,
            path: path.to_path_buf(),
            fields,
        });
    }
    let file: PresetFile = toml::from_str(text).map_err(|e| e.to_string())?;
    if file.format != FORMAT {
        return Err(format!("unsupported preset format {}", file.format));
    }
    let fields = file
        .fields
        .into_iter()
        .map(|r| {
            let target = match (r.block, r.item, r.line) {
                (Some(block), Some(sid), _) => Target::Context {
                    file: r.file,
                    block,
                    kind: if r.delegation {
                        EntryKind::Delegation
                    } else {
                        EntryKind::Item
                    },
                    sid,
                    occurrence: r.occurrence.unwrap_or(0),
                },
                (_, _, Some(line)) => Target::Line { file: r.file, line },
                _ => return Err("field without block/item nor line".to_string()),
            };
            Ok(PresetField {
                target,
                kind: r.field,
                value: r.value,
                original: r.original,
            })
        })
        .collect::<std::result::Result<Vec<_>, String>>()?;
    Ok(Preset {
        name: file.name,
        version: file.format,
        path: path.to_path_buf(),
        fields,
    })
}

fn data_lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines()
        .skip(1)
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
}

// ---------------------------------------------------------------------------
// Application
// ---------------------------------------------------------------------------

/// Resolves the target of a preset field in the workspace.
fn resolve(
    ws: &Workspace,
    index: &HashMap<(usize, &str, EntryKind, &str, u32), EntryId>,
    pf: &PresetField,
) -> Option<FieldId> {
    let file_idx = |path: &str| ws.files().iter().position(|f| f.spec.path == path);
    match &pf.target {
        Target::Context {
            file,
            block,
            kind,
            sid,
            occurrence,
        } => {
            let e = index.get(&(
                file_idx(file)?,
                block.as_str(),
                *kind,
                sid.as_str(),
                *occurrence,
            ))?;
            ws.entry_field(ws.entry(*e), pf.kind)
        }
        Target::Line { file, line } => ws.field_at(FieldLoc {
            file: file_idx(file)?,
            line: *line,
            kind: pf.kind,
        }),
    }
}

/// Remplace les modifications du workspace par celles du preset (une seule
/// undo step). A field that is not found or whose original value has
/// changed is ignored.
pub fn apply(ws: &mut Workspace, p: &Preset) -> ApplyReport {
    let index: HashMap<(usize, &str, EntryKind, &str, u32), EntryId> = ws
        .entry_ids()
        .map(|id| {
            let e = ws.entry(id);
            (
                (
                    e.file,
                    ws.block(e.block).name.as_str(),
                    e.kind,
                    e.sid.as_str(),
                    e.occurrence,
                ),
                id,
            )
        })
        .collect();
    let mut values = Vec::new();
    let mut skipped = 0;
    for pf in &p.fields {
        match resolve(ws, &index, pf) {
            Some(id)
                if pf
                    .original
                    .as_deref()
                    .is_none_or(|o| o == ws.field(id).original()) =>
            {
                values.push((id, pf.value.clone()));
            }
            _ => skipped += 1,
        }
    }
    drop(index);
    let applied = values.len();
    ws.replace_all(&values);
    ApplyReport { applied, skipped }
}
