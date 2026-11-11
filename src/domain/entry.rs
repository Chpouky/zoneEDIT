//! Item / delegation entries and level-1 blocks.

use super::field::FieldId;

/// Identifier of an entry (index in the workspace arena).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntryId(pub u32);

impl EntryId {
    /// Index in the arena.
    pub fn idx(self) -> usize {
        self.0 as usize
    }
}

/// Identifier of a level-1 block (index in the workspace arena).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockId(pub u32);

impl BlockId {
    /// Index in the arena.
    pub fn idx(self) -> usize {
        self.0 as usize
    }
}

/// Nature of an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntryKind {
    /// `ItemPrototypeSID = <sid>`.
    Item,
    /// `ItemGeneratorPrototypeSID = <cible>` : renvoi vers un autre bloc.
    Delegation,
}

/// Bloc de niveau 1 d'un fichier (`Nom : struct.begin`).
#[derive(Debug, Clone)]
pub struct Block {
    /// Index du fichier dans le workspace.
    pub file: usize,
    /// Ligne 1-based du `struct.begin`.
    pub line: usize,
    /// SID du bloc (nom, ou valeur de `SID` pour un bloc `[N]`).
    pub name: String,
    /// Inheritance: `refkey` (parent block) and `refurl` (parent file).
    pub refkey: Option<String>,
    /// Fichier du parent (chemin relatif, ex. `../ItemGeneratorPrototypes.cfg`).
    pub refurl: Option<String>,
    /// Number of delegations (all files) pointing to this block.
    pub delegation_refs: u32,
}

/// Context of an entry, from the deepest scope to the highest.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EntryContext {
    /// Masque des rangs (bit `i` = `config.ranks[i]`) ; 0 = pas de rang.
    pub rank_mask: u32,
    /// Label of the rank ("Newbie", "Veteran, Master", "All ranks").
    pub rank_label: Option<String>,
    /// Real caliber (e.g. `A918`).
    pub caliber: Option<String>,
    /// Section d'arme (ex. `Primary`).
    pub section: Option<String>,
}

impl EntryContext {
    /// Readable label: "Newbie · A045 · Primary".
    pub fn label(&self) -> String {
        [
            self.rank_label.as_deref(),
            self.caliber.as_deref(),
            self.section.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ")
    }
}

/// An entry (item or delegation) of a block.
#[derive(Debug, Clone)]
pub struct Entry {
    /// Nature of the entry (item or delegation).
    pub kind: EntryKind,
    /// SID of the item, or target of the delegation.
    pub sid: String,
    /// Bloc de niveau 1 d'appartenance.
    pub block: BlockId,
    /// Index du fichier dans le workspace.
    pub file: usize,
    /// Contexte (rang, calibre, section).
    pub ctx: EntryContext,
    /// Champs propres (Weight, Chance, Min, Max, AmmoMin/Max…).
    pub fields: Vec<FieldId>,
    /// Shared group fields (MainAmmo, MinSp, MaxSp of the caliber / set).
    pub group_fields: Vec<FieldId>,
    /// Key for grouping the lines (caliber, or rank for a set).
    pub group: Option<String>,
    /// Label of the set ("Set 2 · Bandage") when the group is an itemset.
    pub itemset_label: Option<String>,
    /// Rank of this entry among those with the same (file, block, kind, SID).
    pub occurrence: u32,
    /// Display-only icon hint: SID of the primary item this entry stands
    /// for (the first artifact listed by a world spawner). Never exported.
    pub icon_sid: Option<String>,
}

impl Entry {
    /// The entry is a delegation (`ItemGeneratorPrototypeSID`).
    pub fn is_delegation(&self) -> bool {
        self.kind == EntryKind::Delegation
    }

    /// SID used to look up the row icon: the display hint when present,
    /// otherwise the entry SID itself (placeholder when no icon file).
    pub fn icon_key(&self) -> &str {
        self.icon_sid.as_deref().unwrap_or(&self.sid)
    }

    /// Display name: SID, or `→ target` for a delegation.
    pub fn display_name(&self) -> String {
        match self.kind {
            EntryKind::Item => self.sid.clone(),
            EntryKind::Delegation => format!("→ {}", self.sid),
        }
    }

    /// Champs propres puis champs de groupe.
    pub fn all_fields(&self) -> impl Iterator<Item = FieldId> + '_ {
        self.fields.iter().chain(&self.group_fields).copied()
    }
}
