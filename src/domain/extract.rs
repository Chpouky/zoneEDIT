//! Extraction of the entries and editable fields of a cfg document
//! (state machine over the lexed lines).
//!
//! Entries are opened by the "item" or "delegation" key of the
//! config; the editable fields of their scope are attached to it. Group
//! fields (`scope = "group"`: caliber / consumables set) are attached to the
//! scope that carries them and shared between the entries of the group.
//!
//! Two extraction modes, selected by file role:
//! - `block_entry_roles`: the level-1 block is the entry (anomalies);
//! - `rank_scope_roles`: a scope whose name is a configured rank opens an
//!   entry for the whole block, scoped to that rank (world artifact
//!   spawners). A structureless scope (no rank / caliber / section /
//!   itemset) is *transparent*: the keys under it attach to the entry as if
//!   the scope did not exist (e.g. the `RarityChance` sub-scope).
//!
//! The produced identifiers are LOCAL to the file; the workspace offsets
//! them at assembly.

use crate::cfg::{parse_attrs, Document, LineKind};
use crate::config::Config;

use super::entry::{Block, BlockId, Entry, EntryContext, EntryKind};
use super::field::{Field, FieldId};

/// Result of the extraction of a file (local identifiers).
#[derive(Debug, Default)]
pub struct Extracted {
    /// Level-1 blocks of the file.
    pub blocks: Vec<Block>,
    /// Entries (items and delegations), in order of appearance.
    pub entries: Vec<Entry>,
    /// Editable fields, in order of appearance.
    pub fields: Vec<Field>,
}

/// An open scope on the stack.
#[derive(Debug)]
struct Scope {
    name: String,
    rank: Option<(u32, String)>,
    caliber: Option<String>,
    section: Option<String>,
    itemset: bool,
    /// Group fields seen in this scope.
    group_pending: Vec<FieldId>,
    /// Transparent scope (structureless): the keys under it attach to the
    /// scope above, as if the scope did not exist.
    transparent: bool,
}

impl Scope {
    /// Carries entry structure (section at begin time; the rank / caliber /
    /// itemset flags are set later by keys of the scope). Structureless
    /// scopes are transparent.
    fn has_structure(&self, cfg: &Config) -> bool {
        self.section.is_some()
            || self.itemset
            || self
                .caliber
                .as_deref()
                .is_some_and(|c| c != cfg.parser.none_value)
    }
}

struct Extractor<'a> {
    cfg: &'a Config,
    file: usize,
    stack: Vec<Scope>,
    out: Extracted,
    /// Open entries: (index, depth of the scope containing the entry).
    open: Vec<(usize, usize)>,
    block: Option<BlockId>,
    /// Depth of the open icon-list scope (`ListOfArtifacts`, …), if any.
    icon_list_depth: Option<usize>,
    /// First item listed under the icon-list scope of the current block.
    icon_hint: Option<String>,
}

/// Extracts the blocks, entries and fields of the file at index `file`.
pub fn extract(cfg: &Config, file: usize, doc: &Document) -> Extracted {
    let mut x = Extractor {
        cfg,
        file,
        stack: Vec::new(),
        out: Extracted::default(),
        open: Vec::new(),
        block: None,
        icon_list_depth: None,
        icon_hint: None,
    };
    for (i, line) in doc.lines().iter().enumerate() {
        let n = i + 1;
        match line.kind {
            LineKind::Begin { name, attrs } => {
                let name = doc.span_text(i, name);
                let attrs = doc.span_text(i, attrs);
                x.begin_scope(name, attrs, n);
            }
            LineKind::End => x.end_scope(),
            LineKind::KeyValue { key, value } => {
                let v = doc.span_text(i, value);
                if !v.is_empty() {
                    x.key_value(doc.span_text(i, key), v, n);
                }
            }
            LineKind::Blank | LineKind::Comment | LineKind::Other => {}
        }
    }
    x.out
}

impl Extractor<'_> {
    fn entry_open_at(&self, depth: usize) -> bool {
        self.open.last().is_some_and(|(_, d)| *d == depth)
    }

    fn begin_scope(&mut self, name: &str, attrs: &str, n: usize) {
        let name = if name.is_empty() { "[?]" } else { name };
        if self.stack.is_empty() {
            let attrs = parse_attrs(attrs);
            let attr = |k: &str| {
                attrs
                    .iter()
                    .find(|(a, _)| *a == k)
                    .map(|(_, v)| (*v).to_string())
            };
            self.block = Some(BlockId(self.out.blocks.len() as u32));
            self.out.blocks.push(Block {
                file: self.file,
                line: n,
                name: name.to_string(),
                refkey: attr("refkey"),
                refurl: attr("refurl"),
                delegation_refs: 0,
            });
        }
        // Rank-scope files (world artifact spawners): a scope whose name is
        // a configured rank opens an entry for the whole block, scoped to
        // that rank (the block name is the entry SID).
        if self.is_rank_scope_role() {
            if let Some(i) = self.cfg.rank_index(name) {
                self.stack.push(Scope {
                    name: name.to_string(),
                    rank: Some((1u32 << i, name.to_string())),
                    caliber: None,
                    section: None,
                    itemset: false,
                    group_pending: Vec::new(),
                    transparent: false,
                });
                let b = self.block.expect("rank scope outside a block");
                let sid = self.out.blocks[b.idx()].name.clone();
                self.open_entry(EntryKind::Item, &sid, self.stack.len());
                return;
            }
        }
        // A structureless scope (no rank / caliber / section / itemset) is
        // transparent: the keys under it attach to the scope above (e.g. the
        // `RarityChance` sub-scope of an artifact spawner rank).
        let mut scope = Scope {
            name: name.to_string(),
            rank: None,
            caliber: None,
            section: self.cfg.parser.sections.get(name).cloned(),
            itemset: false,
            group_pending: Vec::new(),
            transparent: false,
        };
        scope.transparent = !scope.has_structure(self.cfg);
        self.stack.push(scope);
        // Icon-list scope (world spawners: `ListOfArtifacts`): its first
        // listed item becomes the display icon hint of the block's rows.
        if self
            .cfg
            .parser
            .icon_list_scope
            .as_deref()
            .is_some_and(|s| s == name)
        {
            self.icon_list_depth = Some(self.stack.len());
        }
        // For the `block_entry_roles` files (anomalies, …) the whole block
        // is the editable unit: open an entry whose SID is the block name;
        // the flat keys directly under the block are its fields.
        if self.stack.len() == 1 && self.is_block_entry_role() {
            self.open_block_entry(name);
        }
    }

    /// Does the current file use the "block = entry" extraction mode?
    fn is_block_entry_role(&self) -> bool {
        self.cfg
            .parser
            .block_entry_roles
            .iter()
            .any(|r| r == &self.cfg.files[self.file].role)
    }

    /// Does the current file use the "rank scope = entry" extraction mode?
    fn is_rank_scope_role(&self) -> bool {
        self.cfg
            .parser
            .rank_scope_roles
            .iter()
            .any(|r| r == &self.cfg.files[self.file].role)
    }

    /// Open an entry for a whole block (no `ItemPrototypeSID`): the block
    /// name is the SID, with an empty context (these files carry no rank /
    /// caliber / section structure).
    fn open_block_entry(&mut self, sid: &str) {
        let Some(b) = self.block else { return };
        self.open.push((self.out.entries.len(), self.stack.len()));
        self.out.entries.push(Entry {
            kind: EntryKind::Item,
            sid: sid.to_string(),
            block: b,
            file: self.file,
            ctx: EntryContext::default(),
            fields: Vec::new(),
            group_fields: Vec::new(),
            group: None,
            itemset_label: None,
            occurrence: 0,
            icon_sid: None,
        });
    }

    fn end_scope(&mut self) {
        // Unbalanced `struct.end`: we only pop if there is a scope.
        if self.stack.pop().is_none() {
            return;
        }
        let depth = self.stack.len() + 1; // depth of the closed scope
        if self.entry_open_at(depth) {
            self.open.pop();
        }
        if self.icon_list_depth == Some(depth) {
            self.icon_list_depth = None;
        }
        if self.stack.is_empty() {
            // Block closed: the icon hint applies to all the entries of the
            // block (one per rank scope for a world spawner).
            if let (Some(hint), Some(b)) = (self.icon_hint.take(), self.block) {
                for e in &mut self.out.entries {
                    if e.block == b && e.icon_sid.is_none() {
                        e.icon_sid = Some(hint.clone());
                    }
                }
            }
            self.block = None;
        }
    }

    fn key_value(&mut self, key: &str, val: &str, n: usize) {
        let depth = self.stack.len();
        if depth == 0 {
            return;
        }
        // Under the icon-list scope: the first listed item (an indexed key,
        // empty values are skipped by the caller) is the block's icon hint.
        if self.icon_list_depth == Some(depth) && self.icon_hint.is_none() {
            if key.starts_with('[') && key.ends_with(']') {
                self.icon_hint = Some(val.to_string());
            }
            return;
        }
        let p = &self.cfg.parser;

        if key == p.item_key || key == p.delegation_key {
            // `empty` = no entry; a 2nd entry key in the same scope
            // (e.g. ItemPrototypeSID + ItemGeneratorPrototypeSID) opens nothing.
            if val != p.empty_value && !self.entry_open_at(depth) {
                let kind = if key == p.item_key {
                    EntryKind::Item
                } else {
                    EntryKind::Delegation
                };
                self.open_entry(kind, val, depth);
            }
            return;
        }
        if key == p.block_sid_key {
            // Renomme un bloc `[N]` de niveau 1 avec son vrai SID.
            if depth == 1 && val != p.empty_value && self.stack[0].name.starts_with('[') {
                self.stack[0].name = val.to_string();
                if let Some(b) = self.block {
                    self.out.blocks[b.idx()].name = val.to_string();
                }
            }
        } else if p.rank_keys.iter().any(|k| k == key) {
            if self.stack[depth - 1].rank.is_none() {
                self.stack[depth - 1].rank = Some(parse_ranks(self.cfg, val));
            }
        } else if key == p.caliber_key {
            if self.stack[depth - 1].caliber.is_none() {
                self.stack[depth - 1].caliber = Some(strip_enum(val).to_string());
            }
        } else if key == p.itemset_key {
            self.stack[depth - 1].itemset = true;
        }

        let Some(kind) = self.cfg.field_kind_for_key(key) else {
            return;
        };
        let id = FieldId(self.out.fields.len() as u32);
        if self.cfg.spec(kind).is_group() {
            // Group field: shared by all the entries of the scope.
            self.out
                .fields
                .push(Field::new(kind, self.file, n, key, val));
            self.stack[depth - 1].group_pending.push(id);
        } else if let Some(&(idx, d)) = self.open.last() {
            let fields = &self.out.fields;
            let entry = &self.out.entries[idx];
            // Under the entry scope, with only transparent (structureless)
            // scopes in between: the flat keys of a rank scope reach the
            // entry even from the `RarityChance` sub-scope.
            let under = depth >= d && (d..depth).all(|i| self.stack[i].transparent);
            if under && !entry.fields.iter().any(|f| fields[f.idx()].kind == kind) {
                self.out
                    .fields
                    .push(Field::new(kind, self.file, n, key, val));
                self.out.entries[idx].fields.push(id);
            }
        }
    }

    /// "Real" caliber (present and different from the "None" value).
    fn real_caliber<'s>(&self, s: &'s Scope) -> Option<&'s str> {
        s.caliber
            .as_deref()
            .filter(|c| *c != self.cfg.parser.none_value)
    }

    fn open_entry(&mut self, kind: EntryKind, sid: &str, depth: usize) {
        // Context: 1st rank / 1st real caliber / 1st section, from the deepest
        // scope to the highest.
        let mut ctx = EntryContext::default();
        for s in self.stack.iter().rev() {
            if ctx.rank_label.is_none() {
                if let Some((mask, label)) = &s.rank {
                    ctx.rank_mask = *mask;
                    ctx.rank_label = Some(label.clone());
                }
            }
            if ctx.caliber.is_none() {
                ctx.caliber = self.real_caliber(s).map(str::to_string);
            }
            if ctx.section.is_none() {
                ctx.section.clone_from(&s.section);
            }
        }
        // Fallback: rank deduced from the BLOCK NAME for files without a
        // Rank/PlayerRank field. Two conventions in the cfgs: `_Veteran`
        // suffix (Stash_Grenade_Veteran) or CamelCase rank in the middle of
        // the name (MercenaryVeteranItemGenerator). Substring search,
        // case-sensitive; no block name contains two ranks.
        // The explicit field (seen in the stack) always wins; a name without
        // rank stays "no rank" (never filtered).
        if ctx.rank_mask == 0 {
            let bname = &self.out.blocks[self.block.expect("entry outside a block").idx()].name;
            let mut mask = 0u32;
            let mut labels = Vec::new();
            for (i, r) in self.cfg.ranks.iter().enumerate() {
                if bname.contains(r.as_str()) {
                    mask |= 1u32 << i;
                    labels.push(r.clone());
                }
            }
            if !labels.is_empty() {
                ctx.rank_mask = mask;
                ctx.rank_label = Some(labels.join(", "));
            }
        }

        let display = match kind {
            EntryKind::Item => sid.to_string(),
            EntryKind::Delegation => format!("→ {sid}"),
        };

        // Group: 1st scope (from the deepest) carrying a set or a real caliber.
        let mut group_fields = Vec::new();
        let mut group = None;
        let mut itemset_label = None;
        let fields = &self.out.fields;
        if let Some(g) = self
            .stack
            .iter()
            .rev()
            .find(|s| s.itemset || self.real_caliber(s).is_some())
        {
            for k in self.cfg.group_kinds() {
                if let Some(id) = g.group_pending.iter().find(|id| fields[id.idx()].kind == k) {
                    group_fields.push(*id);
                }
            }
            if let Some(c) = self.real_caliber(g) {
                group = Some(c.to_string());
            } else {
                // Consumables set: grouped by rank (e.g. "Newbie"),
                // positional label "Set <i> · <item>".
                let label = set_label(&g.name, &display);
                group = Some(ctx.rank_label.clone().unwrap_or_else(|| label.clone()));
                itemset_label = Some(label);
            }
        }

        let block = self.block.expect("entry outside a block (depth >= 1)");
        self.open.push((self.out.entries.len(), depth));
        self.out.entries.push(Entry {
            kind,
            sid: sid.to_string(),
            block,
            file: self.file,
            ctx,
            fields: Vec::new(),
            group_fields,
            group,
            itemset_label,
            occurrence: 0,
            icon_sid: None,
        });
    }
}

/// `ERank::Newbie, ERank::Veteran` → (mask, "Newbie, Veteran"); all
/// configured ranks → "All ranks".
fn parse_ranks(cfg: &Config, val: &str) -> (u32, String) {
    let tokens: Vec<&str> = val
        .split(',')
        .map(|t| strip_enum(t.trim()))
        .filter(|t| !t.is_empty())
        .collect();
    let mask = tokens
        .iter()
        .filter_map(|t| cfg.rank_index(t))
        .fold(0u32, |m, i| m | (1 << i));
    let label = if tokens.len() > 1 && mask == cfg.all_ranks_mask() {
        "All ranks".to_string()
    } else {
        tokens.join(", ")
    };
    (mask, label)
}

/// Label of a set row: "Set <i> · <item>" where <i> is the position
/// of the set in the block (cfg array index, 1-based). Without a readable
/// index (scope `[?]`), simple label "Set · <item>".
fn set_label(scope_name: &str, item: &str) -> String {
    let idx = scope_name
        .trim_start_matches('[')
        .trim_end_matches(']')
        .parse::<usize>()
        .ok();
    match idx {
        Some(i) => format!("Set {} · {item}", i + 1),
        None => format!("Set · {item}"),
    }
}

/// `ERank::Newbie` → `Newbie` ; `EAmmoCaliber::A918` → `A918`.
fn strip_enum(v: &str) -> &str {
    v.rsplit_once("::").map_or(v, |(_, s)| s).trim()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::FieldKind;

    fn run(src: &str) -> Extracted {
        extract(&Config::default_arc(), 0, &Document::from_text(src))
    }

    #[test]
    fn comments_empty_values_and_odd_lines() {
        let x = run("// comment with struct.begin
\tBlock1 : struct.begin
\t\tSID = Block1
\t\tItemGenerator : struct.begin
\t\t\t[0] : struct.begin
\t\t\t\tItemPrototypeSID = TestItem
\t\t\t\tWeight =
\t\t\t\tWeight = 7
\t\t\t\tNote = a = b
\t\t\t\t// Weight = 99
\t\t\t\tFoo : struct
\t\t\t\tItemSetCount = 2f
\t\t\tstruct.end
\t\tstruct.end
\tstruct.end
struct.end
");
        assert_eq!(x.entries.len(), 1);
        assert_eq!(x.entries[0].sid, "TestItem");
        assert_eq!(x.blocks[0].name, "Block1");
        assert_eq!(x.fields.len(), 1);
        assert_eq!(x.fields[0].kind, FieldKind::Weight);
        assert_eq!(x.fields[0].original(), "7");
    }

    /// Extraction in the "rank scope = entry" file (world artifact
    /// spawners, `artifact_spawner` role in the default configuration).
    fn run_artifact(src: &str) -> Extracted {
        let cfg = Config::default_arc();
        let file = cfg
            .files
            .iter()
            .position(|f| f.role == "artifact_spawner")
            .expect("artifact_spawner file configured");
        extract(&cfg, file, &Document::from_text(src))
    }

    #[test]
    fn rank_scopes_open_entries_and_transparent_subscope_attaches() {
        let x = run_artifact(
            "ChunkMeat : struct.begin
   ID = 1
   SID = ChunkMeat
   Newbie : struct.begin
      Count = 2
      Radius = 100.f
      MinCooldown = 1.f
      MaxCooldown = 5.f
      SpawnChanceBase = 10.f
      SpawnChanceBonus = 2.f
      RarityChance : struct.begin
         Common = 50.f
         Rare = 5.f
      struct.end
   struct.end
   Veteran : struct.begin
      Count = 0
   struct.end
struct.end
",
        );
        // One entry per rank scope (the block name is the SID).
        assert_eq!(x.entries.len(), 2);
        assert_eq!(x.entries[0].sid, "ChunkMeat");
        assert_eq!(x.entries[0].ctx.rank_label.as_deref(), Some("Newbie"));
        assert_eq!(x.entries[1].ctx.rank_label.as_deref(), Some("Veteran"));
        // The flat keys AND the keys of the transparent `RarityChance`
        // sub-scope attach to the rank entry.
        let kinds: Vec<FieldKind> = x.entries[0]
            .fields
            .iter()
            .map(|&f| x.fields[f.idx()].kind)
            .collect();
        for k in [
            FieldKind::ArtifactCount,
            FieldKind::Radius,
            FieldKind::MinCooldown,
            FieldKind::MaxCooldown,
            FieldKind::SpawnChanceBase,
            FieldKind::SpawnChanceBonus,
            FieldKind::RarityChanceCommon,
            FieldKind::RarityChanceRare,
        ] {
            assert!(kinds.contains(&k), "missing {k:?}");
        }
        assert_eq!(kinds.len(), 8, "unmapped keys (ID, …) are not read");
    }

    #[test]
    fn first_listed_artifact_is_the_icon_hint_of_all_rank_entries() {
        // The `ListOfArtifacts` scope of a world spawner lists the items it
        // spawns; the first one is the display icon hint of every rank entry
        // of the block (empty values are skipped by the lexer).
        let x = run_artifact(
            "Razor : struct.begin
   ID = 9
   SID = Razor
   Newbie : struct.begin
      Count = 1
   struct.end
   Veteran : struct.begin
      Count = 2
   struct.end
   UseListOfArtifacts = true
   ListOfArtifacts : struct.begin
      [0] = EArtifactRazor
      [1] = EArtifactSoul
   struct.end
struct.end
Empty : struct.begin
   ID = 0
   SID = Empty
   Newbie : struct.begin
      Count = 0
   struct.end
   UseListOfArtifacts = false
struct.end
",
        );
        // Both rank entries of `Razor` carry the hint…
        assert_eq!(x.entries.len(), 3);
        assert_eq!(x.entries[0].icon_sid.as_deref(), Some("EArtifactRazor"));
        assert_eq!(x.entries[1].icon_sid.as_deref(), Some("EArtifactRazor"));
        // …and the lookup key falls back to the SID when there is no list.
        assert_eq!(x.entries[2].icon_sid, None);
        assert_eq!(x.entries[2].icon_key(), "Empty");
        assert_eq!(x.entries[0].icon_key(), "EArtifactRazor");
    }

    #[test]
    fn lowercase_weight_misspelling_is_extracted_as_weight() {
        let x = run("Block1 : struct.begin
   SID = Block1
   Items : struct.begin
      [0] : struct.begin
         ItemPrototypeSID = Medkit
         weight = 3
      struct.end
   struct.end
struct.end
");
        assert_eq!(x.fields.len(), 1);
        assert_eq!(x.fields[0].kind, FieldKind::Weight);
        assert_eq!(x.fields[0].original(), "3");
    }

    #[test]
    fn lowercase_ammo_maxcount_misspelling_is_extracted_as_ammo_max_count() {
        let x = run("Block1 : struct.begin
   SID = Block1
   Items : struct.begin
      [0] : struct.begin
         ItemPrototypeSID = GunAK74_ST
         AmmoMaxcount = 12
      struct.end
   struct.end
struct.end
");
        assert_eq!(x.fields.len(), 1);
        assert_eq!(x.fields[0].kind, FieldKind::AmmoMaxCount);
        assert_eq!(x.fields[0].original(), "12");
    }

    #[test]
    fn sid_renames_indexed_block_and_ranks_are_parsed() {
        let x = run("[3] : struct.begin {refurl=../A.cfg;refkey=Parent}
   SID = RealName
   PlayerRank = ERank::Newbie, ERank::Experienced, ERank::Veteran, ERank::Master
   Items : struct.begin
      [0] : struct.begin
         ItemPrototypeSID = Medkit
         Weight = 1
      struct.end
   struct.end
   Other : struct.begin
      PlayerRank = ERank::Veteran, Master
      [0] : struct.begin
         ItemGeneratorPrototypeSID = Target
         Chance = 2
      struct.end
   struct.end
struct.end
");
        let b = &x.blocks[0];
        assert_eq!(b.name, "RealName");
        assert_eq!(b.refkey.as_deref(), Some("Parent"));
        assert_eq!(b.refurl.as_deref(), Some("../A.cfg"));
        assert_eq!(x.entries[0].ctx.rank_mask, 0b1111);
        assert_eq!(x.entries[0].ctx.rank_label.as_deref(), Some("All ranks"));
        let d = &x.entries[1];
        assert!(d.is_delegation());
        assert_eq!(d.sid, "Target");
        assert_eq!(d.ctx.rank_mask, 0b1100);
        assert_eq!(d.ctx.rank_label.as_deref(), Some("Veteran, Master"));
    }

    #[test]
    fn rank_fallback_from_block_name() {
        // Files without a Rank/PlayerRank field: the rank is deduced from the
        // block name (Stash_Grenade_Veteran, …); a name without suffix stays
        // no rank (never filtered by difficulty).
        let x = run("Stash_Grenade_Veteran : struct.begin
   SID = Stash_Grenade_Veteran
   ItemGenerator : struct.begin
      [0] : struct.begin
         ItemPrototypeSID = GrenadeF1
         Weight = 1
      struct.end
   struct.end
struct.end
[1] : struct.begin
   SID = PlayerItemGenerator
   ItemGenerator : struct.begin
      [0] : struct.begin
         ItemPrototypeSID = Medkit
         Weight = 1
      struct.end
   struct.end
struct.end
");
        assert_eq!(x.entries[0].ctx.rank_mask, 0b0100);
        assert_eq!(x.entries[0].ctx.rank_label.as_deref(), Some("Veteran"));
        assert_eq!(x.entries[1].ctx.rank_mask, 0);
        assert_eq!(x.entries[1].ctx.rank_label, None);
    }

    #[test]
    fn rank_fallback_from_block_name_camelcase() {
        // Second convention: CamelCase rank in the middle of the name, without
        // underscore (MercenaryVeteranItemGenerator, ALLScoutWeaponNewbie…).
        let x = run("MercenaryVeteranItemGenerator : struct.begin
   SID = MercenaryVeteranItemGenerator
   ItemGenerator : struct.begin
      [0] : struct.begin
         ItemPrototypeSID = VestLvl2
         Weight = 1
      struct.end
   struct.end
struct.end
");
        assert_eq!(x.entries[0].ctx.rank_mask, 0b0100);
        assert_eq!(x.entries[0].ctx.rank_label.as_deref(), Some("Veteran"));
    }

    #[test]
    fn explicit_rank_wins_over_block_name() {
        // A present PlayerRank/Rank field wins over the name suffix.
        let x = run("Stash_Med_Master : struct.begin
   SID = Stash_Med_Master
   PlayerRank = ERank::Newbie
   Items : struct.begin
      [0] : struct.begin
         ItemPrototypeSID = Medkit
         Weight = 1
      struct.end
   struct.end
struct.end
");
        assert_eq!(x.entries[0].ctx.rank_mask, 0b0001);
        assert_eq!(x.entries[0].ctx.rank_label.as_deref(), Some("Newbie"));
    }
}
