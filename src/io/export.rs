//! Mod export (ZoneKit): one patch file per modified base file.
//!
//! The user picks the folder of an existing mod (created with ZoneKit or
//! another app) in the OS explorer; the mod name is the name of that
//! folder. Each patch is a plain `.cfg` placed in the prototype subfolder
//! that carries the base file name: `GameData/X.cfg` →
//! `GameData/X/<mod>.cfg`. It only contains the modified blocks: the block
//! root carries the `{bpatch}` keyword (the merge is recursive, so the
//! nested scopes are plain nodes) and the listed fields are replaced, the
//! other fields of the base file are left untouched. The original files are
//! NEVER written, and neither is any other file of the mod: zoneEDIT only
//! writes (or overwrites) its own `<mod>.cfg` patches.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::cfg::{Document, LineKind, ValueEdit};
use crate::domain::{BlockId, FieldId, SourceFile, Workspace};
use crate::error::{Error, Result};

use super::load::fingerprint;

/// Default mod name (fallback when the chosen folder has no usable name).
pub const DEFAULT_MOD_NAME: &str = "zoneedit";

/// A file written by the export.
#[derive(Debug, Clone)]
pub struct Exported {
    /// Short name of the base file.
    pub name: String,
    /// Path of the written patch file.
    pub path: PathBuf,
}

/// Exact content of a file with the modified values of the workspace.
pub fn render(ws: &Workspace, file: usize) -> Result<String> {
    let f = &ws.files()[file];
    let edits: Vec<ValueEdit<'_>> = ws
        .dirty_fields()
        .map(|id| ws.field(id))
        .filter(|fd| fd.file == file)
        .map(|fd| ValueEdit {
            line: fd.line,
            value: fd.value(),
        })
        .collect();
    f.doc.render(&edits).map_err(|e| Error::NoValueOnLine {
        name: f.name().to_string(),
        line: e.0,
    })
}

/// Sanitize a mod name for use as a file name: keeps `[A-Za-z0-9_-]`,
/// replaces the other characters, falls back to the default when empty.
pub fn sanitize_mod_name(name: &str) -> String {
    let s: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if s.trim().is_empty() {
        DEFAULT_MOD_NAME.to_string()
    } else {
        s
    }
}

/// Subfolder of the export that receives the patches of a base file:
/// `GameData/X.cfg` → `GameData/X` (parent + file stem, recursive).
pub fn patch_subdir(spec_path: &str) -> PathBuf {
    let rel = Path::new(spec_path);
    let stem = rel.file_stem().and_then(|s| s.to_str()).unwrap_or("patch");
    match rel.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.join(stem),
        _ => PathBuf::from(stem),
    }
}

/// Relative path of the patch file of a base file:
/// `GameData/X.cfg` → `GameData/X/<mod>.cfg`.
pub fn patch_rel_path(spec_path: &str, mod_name: &str) -> PathBuf {
    patch_subdir(spec_path).join(format!("{mod_name}.cfg"))
}

/// Renders the patch file of a base file: one node per modified block.
/// Each modified field is re-emitted at its real depth: the scopes between
/// the block and the field are rebuilt as nested plain `struct.begin`
/// nodes — the `{bpatch}` keyword is carried only by the block root, because
/// the merge is recursive from the root (article 172) — so only the listed
/// fields are replaced and every other field of the base file is left
/// untouched. The node is addressed by its
/// name in the base file; the reference attributes (`{refkey=…}`,
/// `{refurl=…}`) are not re-emitted.
pub fn render_patch(ws: &Workspace, file: usize) -> Result<String> {
    let f = &ws.files()[file];

    // Modified fields grouped by block.
    let mut by_block: Vec<(BlockId, Vec<FieldId>)> = Vec::new();
    let mut pos: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for id in ws.dirty_fields() {
        let fd = ws.field(id);
        if fd.file != file {
            continue;
        }
        let bid = match ws.owner(id) {
            Some(e) => ws.entry(e).block,
            None => {
                return Err(Error::NoBlock {
                    name: f.name().to_string(),
                    line: fd.line,
                })
            }
        };
        let p = *pos.entry(bid.idx()).or_insert_with(|| {
            by_block.push((bid, Vec::new()));
            by_block.len() - 1
        });
        by_block[p].1.push(id);
    }

    // Blocks in file order (line of the struct.begin).
    by_block.sort_by_key(|(b, _)| ws.block(*b).line);

    let mut out = String::new();
    for (bid, fields) in by_block {
        let b = ws.block(bid);
        let paths = scope_paths(&f.doc, b.line);
        let mut tree = PatchNode::default();
        for id in fields {
            let fd = ws.field(id);
            let Some(path) = paths.get(&fd.line) else {
                return Err(Error::NoBlock {
                    name: f.name().to_string(),
                    line: fd.line,
                });
            };
            // The key is re-emitted exactly as written in the base file
            // (the merge matches keys by name: the canonical key of an
            // aliased field would add a new key instead of replacing it).
            tree.insert(path, (fd.key().to_string(), fd.value().to_string()));
        }
        tree.emit(&mut out, &original_node_name(&f.doc, b.line), 0, true);
    }
    Ok(out)
}

/// Name of a level-1 node as written in the base file (no attributes):
/// `[156] : struct.begin {refkey=[0]}` → `[156]`.
fn original_node_name(doc: &Document, line: usize) -> String {
    doc.line_text(line - 1)
        .split(':')
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}

/// Maps every `KeyValue` line of the file to the path of scope names
/// between the level-1 block starting at `block_line` and that line
/// (empty path = field directly in the block).
fn scope_paths(doc: &Document, block_line: usize) -> BTreeMap<usize, Vec<String>> {
    let mut out = BTreeMap::new();
    let mut stack: Vec<String> = Vec::new();
    let mut active = false;
    let mut depth = 0usize;
    for (i, line) in doc.lines().iter().enumerate() {
        let n = i + 1;
        match line.kind {
            LineKind::Begin { name, .. } => {
                if depth == 0 {
                    // Level-1 block: only ours is active.
                    active = n == block_line;
                    stack.clear();
                } else if active {
                    stack.push(doc.span_text(i, name).to_string());
                }
                depth += 1;
            }
            LineKind::End => {
                if active && depth >= 1 {
                    stack.pop();
                }
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    active = false;
                }
            }
            LineKind::KeyValue { .. } => {
                if active {
                    out.insert(n, stack.clone());
                }
            }
            LineKind::Blank | LineKind::Comment | LineKind::Other => {}
        }
    }
    out
}

/// Nested tree of patch nodes: the root carries `{bpatch}`, the nested
/// scopes are plain (the merge is recursive); the children of a node are its
/// sub-scopes, the fields its own modified values.
#[derive(Debug, Default)]
struct PatchNode {
    fields: Vec<(String, String)>,
    children: BTreeMap<String, PatchNode>,
}

impl PatchNode {
    fn insert(&mut self, path: &[String], field: (String, String)) {
        match path.split_first() {
            Some((head, rest)) => self
                .children
                .entry(head.clone())
                .or_default()
                .insert(rest, field),
            None => self.fields.push(field),
        }
    }

    fn emit(&self, out: &mut String, name: &str, indent: usize, bpatch: bool) {
        let pad = "    ".repeat(indent);
        // The keyword is required only on the root: the merge is recursive.
        let kw = if bpatch { " {bpatch}" } else { "" };
        out.push_str(&format!("{pad}{name} : struct.begin{kw}\n"));
        for (key, value) in &self.fields {
            out.push_str(&format!("{pad}    {key} = {value}\n"));
        }
        for (child, node) in &self.children {
            node.emit(out, child, indent + 1, false);
        }
        out.push_str(&format!("{pad}struct.end\n"));
    }
}

/// Exports the modified files as ZoneKit patches into `out_dir` (the
/// `GameData` folder of the user's mod), after checking the integrity of
/// the sources and the validity of the values. Only the own `<mod>.cfg`
/// patch files are written (overwritten on re-export): the other files of
/// the mod are never touched.
pub fn export(ws: &Workspace, out_dir: &Path, mod_name: &str) -> Result<Vec<Exported>> {
    let mod_name = sanitize_mod_name(mod_name);

    // 1. No source file must have moved since the load.
    if let Some(f) = ws
        .files()
        .iter()
        .find(|f| fingerprint(&f.path) != Some(f.fingerprint))
    {
        return Err(Error::Stale {
            name: f.name().to_string(),
        });
    }

    // 2. Global validation before writing anything.
    let errs = ws.validation_errors();
    if !errs.is_empty() {
        return Err(Error::Validation(errs));
    }

    // 3. Full in-memory rendering before any writing.
    let mut dirty_files = vec![false; ws.files().len()];
    for id in ws.dirty_fields() {
        dirty_files[ws.field(id).file] = true;
    }
    let mut rendered: Vec<(usize, String)> = Vec::new();
    for (i, &is_dirty) in dirty_files.iter().enumerate() {
        if is_dirty {
            rendered.push((i, render_patch(ws, i)?));
        }
    }

    // 4. Writing: only the own patch files (created or overwritten).
    let mkdir = |p: &Path| {
        std::fs::create_dir_all(p).map_err(|source| Error::Write {
            path: p.to_path_buf(),
            source,
        })
    };
    mkdir(out_dir)?;
    let mut written = Vec::with_capacity(rendered.len());
    for (i, content) in rendered {
        let f: &SourceFile = &ws.files()[i];
        let path = out_dir.join(patch_rel_path(&f.spec.path, &mod_name));
        if let Some(parent) = path.parent() {
            mkdir(parent)?;
        }
        write_atomic(&path, content.as_bytes()).map_err(|source| Error::Write {
            path: path.clone(),
            source,
        })?;
        written.push(Exported {
            name: f.name().to_string(),
            path,
        });
    }
    Ok(written)
}

/// Atomic writing: temporary file then rename (a crash leaves
/// no half-written file). The `.tmp` is always removed.
pub(crate) fn write_atomic(path: &Path, content: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let name = path
        .file_name()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "no file name"))?;
    let tmp = path.with_file_name(format!("{}.tmp", name.to_string_lossy()));
    let res = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(content)?;
        f.sync_all()?;
        std::fs::rename(&tmp, path)
    })();
    if res.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize() {
        assert_eq!(sanitize_mod_name(""), DEFAULT_MOD_NAME);
        assert_eq!(sanitize_mod_name("   "), DEFAULT_MOD_NAME);
        assert_eq!(sanitize_mod_name("My Mod!"), "MyMod");
        assert_eq!(sanitize_mod_name("zoneedit"), "zoneedit");
    }

    #[test]
    fn scope_paths_follow_the_nested_structs() {
        let src = "[2] : struct.begin {refkey=[0]}
   SID = X
   ItemGenerator : struct.begin
      [0] : struct.begin
         PossibleItems : struct.begin
            [0] : struct.begin
               ItemPrototypeSID = Medkit
               Weight = 1
            struct.end
         struct.end
      struct.end
   struct.end
struct.end
[3] : struct.begin
   SID = Y
   Weight = 2
struct.end
";
        let doc = Document::from_text(src);
        let paths = scope_paths(&doc, 1);
        // The Weight of the item: full path from the block.
        let weight_line = doc
            .lines()
            .iter()
            .enumerate()
            .find(|(i, l)| {
                matches!(l.kind, LineKind::KeyValue { .. })
                    && doc.line_text(*i).contains("Weight = 1")
            })
            .map(|(i, _)| i + 1)
            .unwrap();
        assert_eq!(
            paths.get(&weight_line),
            Some(&vec![
                "ItemGenerator".to_string(),
                "[0]".to_string(),
                "PossibleItems".to_string(),
                "[0]".to_string()
            ])
        );
        // Only the lines of block [2] are mapped (not block [3]).
        assert!(paths.values().all(|p| p.len() <= 4));
        assert_eq!(paths.len(), 3); // ItemPrototypeSID + Weight + SID
    }

    #[test]
    fn patch_node_nests_scopes_and_keeps_field_order() {
        let mut t = PatchNode::default();
        t.insert(
            &[
                "ItemGenerator".to_string(),
                "[0]".to_string(),
                "PossibleItems".to_string(),
                "[0]".to_string(),
            ],
            ("Weight".to_string(), "1".to_string()),
        );
        t.insert(
            &[
                "ItemGenerator".to_string(),
                "[0]".to_string(),
                "PossibleItems".to_string(),
                "[0]".to_string(),
            ],
            ("MaxCount".to_string(), "4".to_string()),
        );
        let mut out = String::new();
        t.emit(&mut out, "[2]", 0, true);
        assert_eq!(
            out,
            concat!(
                "[2] : struct.begin {bpatch}\n",
                "    ItemGenerator : struct.begin\n",
                "        [0] : struct.begin\n",
                "            PossibleItems : struct.begin\n",
                "                [0] : struct.begin\n",
                "                    Weight = 1\n",
                "                    MaxCount = 4\n",
                "                struct.end\n",
                "            struct.end\n",
                "        struct.end\n",
                "    struct.end\n",
                "struct.end\n"
            )
        );
    }

    #[test]
    fn original_name_strips_the_attributes() {
        let doc = Document::from_text(
            "[156] : struct.begin {refurl=../A.cfg;refkey=[0]}\n   Weight = 1\nstruct.end\n",
        );
        assert_eq!(original_node_name(&doc, 1), "[156]");
    }

    #[test]
    fn patch_paths() {
        assert_eq!(
            patch_subdir("GameData/ItemGeneratorPrototypes.cfg"),
            PathBuf::from("GameData/ItemGeneratorPrototypes")
        );
        assert_eq!(
            patch_subdir("GameData/ItemGeneratorPrototypes/DynamicItemGenerator.cfg"),
            PathBuf::from("GameData/ItemGeneratorPrototypes/DynamicItemGenerator")
        );
        assert_eq!(
            patch_rel_path("GameData/StashPrototypes.cfg", "zoneedit"),
            PathBuf::from("GameData/StashPrototypes/zoneedit.cfg")
        );
    }
}
