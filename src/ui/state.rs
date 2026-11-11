//! Editor state: filters, views, selections, presets.

use egui::{Color32, Rect};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use crate::config::CategoryId;
use crate::domain::{BlockId, EntryContext, EntryId, FieldId, Workspace};
use crate::io::presets::Preset;

// ---------------------------------------------------------------------------
// Filters
// ---------------------------------------------------------------------------

/// Does the rank (mask of the entry) match the filtered difficulties?
/// No filter active (`ranks == 0` and `no_rank == false`) = all; otherwise
/// union of the filters: an entry is visible if one of its ranks is
/// selected (bitmask, a composite rank matches each of its
/// components) or if "No Rank" is selected and it has no
/// rank (neither a Rank/PlayerRank field, nor a rank in the block name).
pub(crate) fn matches_difficulty(ctx: &EntryContext, ranks: u32, no_rank: bool) -> bool {
    if ranks == 0 && !no_rank {
        return true;
    }
    (ctx.rank_mask & ranks) != 0 || (no_rank && ctx.rank_mask == 0)
}

/// The rank of an entry, if it has one (possibly composite).
pub(crate) fn entry_rank(ctx: &EntryContext) -> Option<&str> {
    ctx.rank_label.as_deref()
}

// ---------------------------------------------------------------------------
// Editor state
// ---------------------------------------------------------------------------

/// Action expected in the confirmation window.
#[derive(Clone)]
pub(crate) enum ConfirmAction {
    Reload,
    LoadProject(PathBuf),
    /// Opening a folder chosen in the OS native picker
    /// (Folder menu → Open folder…).
    OpenFolder(PathBuf),
    ApplyPreset(usize),
}

/// Menu open in the topbar.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum TopMenu {
    Project,
    File,
}

/// Open modal window (only one at a time).
#[derive(Clone)]
pub(crate) enum Modal {
    /// Topbar dropdown menu.
    Menu(TopMenu),
    /// Diff window.
    Diff,
    /// Validation errors panel.
    Errors,
    /// Redirect window.
    Redirect(String),
    /// Confirmation window.
    Confirm(ConfirmAction),
    /// Batch window (config `[[batch]]`): popup above the button
    /// (`anchor`), `inputs` = field entries, `meas_h` = height of the
    /// popup measured on the previous frame (0 = estimate).
    Batch {
        idx: usize,
        inputs: Vec<String>,
        anchor: Rect,
        meas_h: f32,
    },
}

/// View state (filters + expanded/collapsed).
pub(crate) struct ViewState {
    /// `None` = all files; otherwise index into `Workspace::files`.
    pub(crate) file_filter: Option<usize>,
    /// Selected sidebar section (index in `Config::sections()`).
    pub(crate) section: usize,
    /// Selected spawn source (exactly one).
    pub(crate) category: CategoryId,
    pub(crate) search: String,
    /// Selected ranks (bit `i` = `Config::ranks[i]`); `0` = all.
    /// Cumulative filters (union), like `trade` and `no_rank`.
    pub(crate) difficulty: u32,
    /// "No Rank" filter: entries with no rank (neither a
    /// Rank/PlayerRank field, nor a rank in the block name).
    pub(crate) no_rank: bool,
    /// Only show entries containing "trade" (name, block or
    /// context, case-insensitive): trader inventories, etc.
    pub(crate) trade: bool,
    /// Expanded blocks. Empty = everything collapsed by default.
    pub(crate) expanded: HashSet<BlockId>,
    /// Collapsed ranks (sub-dropdowns): (block, rank label).
    pub(crate) rank_collapsed: HashSet<(BlockId, String)>,
    /// Show the "N refs" badges on block headers
    /// (delegation targets). Disabled by default.
    pub(crate) show_refs: bool,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            file_filter: None,
            // First section of the configuration (ITEM SPAWNS by default).
            section: 0,
            // First category of the configuration (MedCrate by default).
            category: CategoryId(0),
            search: String::new(),
            difficulty: 0,
            no_rank: false,
            trade: false,
            expanded: HashSet::new(),
            rank_collapsed: HashSet::new(),
            show_refs: false,
        }
    }
}

/// Cell editing state (ephemeral TextEdit).
#[derive(Default)]
pub(crate) struct EditingState {
    /// Field being edited (ephemeral TextEdit).
    pub(crate) cell: Option<FieldId>,
    /// Generation of the editing session (incremented on each open).
    pub(crate) gen: u64,
    /// Last generation actually drawn (detects the 1st frame).
    pub(crate) drawn_gen: u64,
    /// Buffer of the current TextEdit (persisted across frames).
    pub(crate) text: String,
}

/// Batch selection: identifier of the 1st field of each selected row.
#[derive(Default)]
pub(crate) struct SelectionState {
    pub(crate) batch: HashSet<FieldId>,
    /// Last clicked row (for Shift+click extension, computed in
    /// DISPLAY order — the original shift+click worked in selection
    /// order, bug fixed in phase 6).
    pub(crate) last_clicked: Option<EntryId>,
}

/// Presets state.
#[derive(Default)]
pub(crate) struct PresetState {
    pub(crate) list: Vec<Preset>,
    /// 0 = Vanilla, `1 + i` = `list[i]`.
    pub(crate) choice: usize,
    pub(crate) open: bool,
    pub(crate) new_name: String,
    /// Actual height of the presets menu measured on the previous frame
    /// (0 = not yet measured → conservative estimate). The menu opens
    /// UPWARD from the dropdown: its position depends on this height.
    pub(crate) menu_h: f32,
}

/// Actions pushed by the bars / menus / shortcuts during the frame.
/// They are applied in batch **after** drawing (`show_editor`), so that
/// a button, a menu item or a shortcut trigger the same code path.
/// (Cell editing is still handled live, it is too fine-grained.)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    /// Reload files from disk (confirm if pending modifications).
    Reload,
    /// Export the mod.
    Export,
    /// Open the diff view.
    ShowDiff,
    /// Open the list of errors.
    ShowErrors,
    /// Reset the selected rows (no effect if nothing is selected).
    ResetSelection,
    /// Reset all values of the workspace.
    ResetAll,
    /// Batch window of the config (`idx` = index in `Config::batch`).
    Batch { idx: usize, anchor: Rect },
}

#[derive(Default)]
pub(crate) struct EditorState {
    pub(crate) view: ViewState,
    pub(crate) editing: EditingState,
    pub(crate) selection: SelectionState,
    pub(crate) presets: PresetState,
    /// Open modal window (only one at a time).
    pub(crate) modal: Option<Modal>,
    /// Sidebar section dropdown open ("ITEM SPAWNS" / "STATS"…).
    pub(crate) section_menu: bool,
    /// Ephemeral message shown on the right of the status bar.
    pub(crate) msg: Option<(bool, String)>,
    /// Time (ctx.input time) of the first frame where the message is visible;
    /// `None` = the expiration countdown has not started.
    pub(crate) msg_since: Option<f64>,
    /// Actions accumulated during the frame, applied after drawing.
    pub(crate) pending_actions: Vec<Action>,
    /// Previous frame time in milliseconds (plan 4.1): measured in
    /// `ZoneEditApp::update`, shown in the status bar.
    pub(crate) frame_ms: f32,
    /// Cache of the mod project list (Folder menu): `(root, list)`.
    /// Re-computed when opening the menu or when the root changes (plan 4.3)
    /// — no more `read_dir` + `canonicalize` on every frame.
    pub(crate) project_cache: Option<(PathBuf, Vec<PathBuf>)>,
}

impl EditorState {
    /// Push an action to apply at the end of the frame.
    pub(crate) fn push_action(&mut self, a: Action) {
        self.pending_actions.push(a);
    }

    /// List of mod projects (Folder menu), served from the cache: the disk
    /// scan only runs on the first access after opening the menu or
    /// changing the root (plan 4.3).
    pub(crate) fn projects_for(&mut self, ws: &Workspace) -> Vec<PathBuf> {
        match &self.project_cache {
            Some((r, list)) if r == &ws.root => list.clone(),
            _ => {
                let list = crate::io::load::scan_projects(&ws.root, ws.config());
                self.project_cache = Some((ws.root.clone(), list.clone()));
                list
            }
        }
    }

    /// Clears the queue of actions (called before batch application).
    pub(crate) fn take_actions(&mut self) -> Vec<Action> {
        std::mem::take(&mut self.pending_actions)
    }

    /// Resets the view (preserves the list of loaded presets).
    pub(crate) fn reset_view(&mut self) {
        let loaded = std::mem::take(&mut self.presets.list);
        *self = Self::default();
        self.presets.list = loaded;
    }
}

// ---------------------------------------------------------------------------
// Derived state (one pass per frame, step 3.8)
// ---------------------------------------------------------------------------

/// Derived state for the current frame, computed only once at the top of
/// `show_editor` (`compute_derived`): the drawing code no longer re-scans the
/// workspace per widget (counters, "select all", selectable keys
/// pre-grouped by block).
#[derive(Default)]
pub(crate) struct Derived {
    /// Identifiers of the visible entries (no `Entry` clone).
    pub visible_idx: Vec<EntryId>,
    /// Visible entries grouped by block: the "select all" boxes
    /// and the ranks only scan their block.
    /// The length of this map = number of visible generators.
    pub block_entries: HashMap<BlockId, Vec<EntryId>>,
    /// Modified (dirty) fields.
    pub dirty: usize,
    /// Validation errors: counting only, without building the `String`s.
    pub n_err: usize,
    /// Sidebar counters, depending on the "File" filter.
    pub cat_counts: Vec<(CategoryId, usize)>,
    /// Highlight tint of a visible delegation row.
    pub delegation_tint: HashMap<EntryId, Color32>,
    /// Tint applied to all rows of a block (header + rows):
    /// the source and target blocks of a visible delegation share the
    /// same tint (see `compute_derived`).
    pub block_tint: HashMap<BlockId, Color32>,
}
