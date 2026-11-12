//! Workspace: the complete state of a session (files, blocks, entries, fields)
//! and indexes precomputed at load time.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

use crate::cfg::Document;
use crate::config::{CategoryId, Config, FieldSpec, FileSpec};

use super::classify::classify;
use super::edit::History;
use super::entry::{Block, BlockId, Entry, EntryId, EntryKind};
use super::extract::Extracted;
use super::field::{Field, FieldId, FieldKind, FieldLoc};

/// Fingerprint of a file on disk (external modification detection).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fingerprint {
    /// Last modification date.
    pub mtime: SystemTime,
    /// Size in bytes.
    pub size: u64,
}

/// A loaded cfg file.
#[derive(Debug)]
pub struct SourceFile {
    /// Description of the file (name, role) coming from the config.
    pub spec: FileSpec,
    /// Absolute path.
    pub path: PathBuf,
    /// In-memory document (source text + indexes).
    pub doc: Document,
    /// Fingerprint at load time.
    pub fingerprint: Fingerprint,
}

impl SourceFile {
    /// Short name of the file (from the config).
    pub fn name(&self) -> &str {
        &self.spec.name
    }
}

/// Delegation target without a matching block.
#[derive(Debug, Clone)]
pub struct Orphan {
    /// Name of the missing target.
    pub target: String,
    /// Delegations that point to this target.
    pub entries: Vec<EntryId>,
}

/// Global counter of loads (identity of a workspace for the caches).
static GENERATION: AtomicU64 = AtomicU64::new(1);

/// All the editable state of the session.
#[derive(Debug)]
pub struct Workspace {
    /// Loaded game root.
    pub root: PathBuf,
    /// Folder chosen by the user in the explorer (name displayed
    /// in the "Folder" dropdown). Can be an ancestor of `root`
    /// (e.g. the ZoneKit parent of `GameLite`). Default: `root`.
    pub chosen_dir: PathBuf,
    pub(super) config: Arc<Config>,
    pub(super) files: Vec<SourceFile>,
    pub(super) blocks: Vec<Block>,
    pub(super) entries: Vec<Entry>,
    pub(super) fields: Vec<Field>,
    // Indexes (immutable after load).
    loc_index: HashMap<FieldLoc, FieldId>,
    field_owner: Vec<Option<EntryId>>,
    categories: Vec<Vec<CategoryId>>,
    search_keys: Vec<String>,
    orphans: Vec<Orphan>,
    generation: u64,
    // Editing state.
    pub(super) revision: u64,
    pub(super) dirty: usize,
    pub(super) history: History,
}

impl Workspace {
    /// Assembles the extracted files (local → global identifiers) and
    /// computes all the indexes.
    pub fn assemble(
        root: PathBuf,
        config: Arc<Config>,
        parts: Vec<(SourceFile, Extracted)>,
    ) -> Workspace {
        let mut files = Vec::with_capacity(parts.len());
        let mut blocks = Vec::new();
        let mut entries = Vec::new();
        let mut fields = Vec::new();
        for (file, x) in parts {
            let (b0, f0) = (blocks.len() as u32, fields.len() as u32);
            files.push(file);
            blocks.extend(x.blocks);
            fields.extend(x.fields);
            entries.extend(x.entries.into_iter().map(|mut e| {
                e.block = BlockId(e.block.0 + b0);
                for id in e.fields.iter_mut().chain(e.group_fields.iter_mut()) {
                    id.0 += f0;
                }
                e
            }));
        }

        let mut ws = Workspace {
            root: root.clone(),
            chosen_dir: root,
            config,
            files,
            blocks,
            entries,
            fields,
            loc_index: HashMap::new(),
            field_owner: Vec::new(),
            categories: Vec::new(),
            search_keys: Vec::new(),
            orphans: Vec::new(),
            generation: GENERATION.fetch_add(1, Ordering::Relaxed),
            revision: 0,
            dirty: 0,
            history: History::default(),
        };
        ws.build_indexes();
        ws
    }

    fn build_indexes(&mut self) {
        self.loc_index = self
            .fields
            .iter()
            .enumerate()
            .map(|(i, f)| (f.loc(), FieldId(i as u32)))
            .collect();

        // Owner (1st entry) of each field.
        self.field_owner = vec![None; self.fields.len()];
        for (i, e) in self.entries.iter().enumerate() {
            for f in e.all_fields() {
                self.field_owner[f.idx()].get_or_insert(EntryId(i as u32));
            }
        }

        // Occurrence of each entry among its namesakes (presets).
        let mut seen: HashMap<(usize, &str, EntryKind, &str), u32> = HashMap::new();
        let mut occ = Vec::with_capacity(self.entries.len());
        for e in &self.entries {
            let n = seen
                .entry((
                    e.file,
                    self.blocks[e.block.idx()].name.as_str(),
                    e.kind,
                    e.sid.as_str(),
                ))
                .or_insert(0);
            occ.push(*n);
            *n += 1;
        }
        for (e, o) in self.entries.iter_mut().zip(occ) {
            e.occurrence = o;
        }

        // Categories and search keys.
        self.categories = self
            .entries
            .iter()
            .map(|e| {
                classify(
                    &self.config,
                    &self.files[e.file].spec,
                    &self.blocks[e.block.idx()].name,
                    e,
                    &self.fields,
                )
            })
            .collect();
        self.search_keys = self
            .entries
            .iter()
            .map(|e| {
                format!(
                    "{}\n{}\n{}",
                    e.display_name(),
                    self.blocks[e.block.idx()].name,
                    e.ctx.label()
                )
                .to_lowercase()
            })
            .collect();

        // Delegations: references per block and orphan targets.
        let mut refs: HashMap<&str, Vec<EntryId>> = HashMap::new();
        for (i, e) in self.entries.iter().enumerate() {
            if e.is_delegation() {
                refs.entry(e.sid.as_str())
                    .or_default()
                    .push(EntryId(i as u32));
            }
        }
        for b in &mut self.blocks {
            b.delegation_refs = refs.get(b.name.as_str()).map_or(0, |v| v.len() as u32);
        }
        let names: std::collections::HashSet<&str> =
            self.blocks.iter().map(|b| b.name.as_str()).collect();
        let mut orphans: Vec<Orphan> = refs
            .into_iter()
            .filter(|(t, _)| !names.contains(t))
            .map(|(t, entries)| Orphan {
                target: t.to_string(),
                entries,
            })
            .collect();
        orphans.sort_by(|a, b| a.target.cmp(&b.target));
        self.orphans = orphans;
    }

    // -----------------------------------------------------------------------
    // Accessors
    // -----------------------------------------------------------------------

    /// Configuration in effect for this session.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Light clone of the configuration (Arc).
    pub fn config_arc(&self) -> Arc<Config> {
        self.config.clone()
    }

    /// Schema of a field type.
    pub fn spec(&self, kind: FieldKind) -> &FieldSpec {
        self.config.spec(kind)
    }

    /// Loaded files, in the order of the config.
    pub fn files(&self) -> &[SourceFile] {
        &self.files
    }

    /// All level-1 blocks.
    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    /// Block by identifier.
    pub fn block(&self, id: BlockId) -> &Block {
        &self.blocks[id.idx()]
    }

    /// All entries.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Entry by identifier.
    pub fn entry(&self, id: EntryId) -> &Entry {
        &self.entries[id.idx()]
    }

    /// All entry identifiers.
    pub fn entry_ids(&self) -> impl Iterator<Item = EntryId> {
        (0..self.entries.len() as u32).map(EntryId)
    }

    /// All fields.
    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    /// Field by identifier.
    pub fn field(&self, id: FieldId) -> &Field {
        &self.fields[id.idx()]
    }

    /// All field identifiers.
    pub fn field_ids(&self) -> impl Iterator<Item = FieldId> {
        (0..self.fields.len() as u32).map(FieldId)
    }

    /// Field at a given location.
    pub fn field_at(&self, loc: FieldLoc) -> Option<FieldId> {
        self.loc_index.get(&loc).copied()
    }

    /// First entry that carries the field.
    pub fn owner(&self, id: FieldId) -> Option<EntryId> {
        self.field_owner[id.idx()]
    }

    /// Categories of the entry (empty if unclassified; several for the
    /// `multi_category` files).
    pub fn categories_of(&self, id: EntryId) -> &[CategoryId] {
        &self.categories[id.idx()]
    }

    /// The entry belongs to the category (multi-category files can match
    /// several rows of the sidebar).
    pub fn in_category(&self, id: EntryId, cat: CategoryId) -> bool {
        self.categories[id.idx()].contains(&cat)
    }

    /// Lowercase search key (name, block, context).
    pub fn search_key(&self, id: EntryId) -> &str {
        &self.search_keys[id.idx()]
    }

    /// Delegation targets without a matching block.
    pub fn orphans(&self) -> &[Orphan] {
        &self.orphans
    }

    /// Identity of the load (changes on each reload).
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Revision of the values (changes on each modification).
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Number of modified fields.
    pub fn dirty_count(&self) -> usize {
        self.dirty
    }

    /// Modified fields, in (file, line) order.
    pub fn dirty_fields(&self) -> impl Iterator<Item = FieldId> + '_ {
        self.field_ids().filter(|id| self.field(*id).is_dirty())
    }

    /// `struct.begin` / `struct.end` balanced in each file.
    pub fn struct_balanced(&self) -> bool {
        self.files.iter().all(|f| f.doc.is_balanced())
    }

    // -----------------------------------------------------------------------
    // Queries on the entries
    // -----------------------------------------------------------------------

    /// Own field of the entry for a type.
    pub fn own_field(&self, e: &Entry, kind: FieldKind) -> Option<FieldId> {
        e.fields
            .iter()
            .copied()
            .find(|f| self.field(*f).kind == kind)
    }

    /// Field (own then group) of the entry for a type.
    pub fn entry_field(&self, e: &Entry, kind: FieldKind) -> Option<FieldId> {
        e.all_fields().find(|f| self.field(*f).kind == kind)
    }

    /// First configured (min, max) pair present on the entry.
    pub fn count_pair(&self, e: &Entry) -> Option<(FieldId, FieldId)> {
        self.config
            .pairs
            .iter()
            .find_map(|&(a, b)| Some((self.own_field(e, a)?, self.own_field(e, b)?)))
    }

    /// Configured (min, max) pairs present on the entry.
    pub fn count_pairs<'a>(
        &'a self,
        e: &'a Entry,
    ) -> impl Iterator<Item = (FieldId, FieldId)> + 'a {
        self.config
            .pairs
            .iter()
            .filter_map(move |&(a, b)| Some((self.own_field(e, a)?, self.own_field(e, b)?)))
    }

    /// 1-based line of the entry (first field, else block line).
    pub fn entry_line(&self, e: &Entry) -> usize {
        e.all_fields()
            .next()
            .map_or_else(|| self.block(e.block).line, |f| self.field(f).line)
    }

    /// Non-empty categories with their entry count, in the order of the
    /// config; `file = Some(i)`: limited to one file.
    pub fn category_counts(&self, file: Option<usize>) -> Vec<(CategoryId, usize)> {
        let mut counts = vec![0usize; self.config.categories.len()];
        for (e, cats) in self.entries.iter().zip(&self.categories) {
            for &c in cats {
                if file.is_none_or(|f| f == e.file) {
                    counts[c.idx()] += 1;
                }
            }
        }
        counts
            .into_iter()
            .enumerate()
            .filter(|(_, n)| *n > 0)
            .map(|(i, n)| (CategoryId(i as u16), n))
            .collect()
    }

    /// Unique block names, sorted ("Redirect" window).
    pub fn block_names(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self.blocks.iter().map(|b| b.name.as_str()).collect();
        v.sort_unstable();
        v.dedup();
        v
    }
}
