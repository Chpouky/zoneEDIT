//! Main table: columns, block/rank/group/item rows, cells.

use std::collections::HashMap;

use egui::{pos2, Align2, Color32, FontId, Id, Sense, Ui};

use crate::config::CategoryId;
use crate::domain::{BlockId, EntryId, FieldId, FieldKind, Workspace};
use crate::ui::icons::icon_for;
use crate::ui::layout::RowLayout;
use crate::ui::metrics::{FONT_BODY, FONT_BUTTON, ROW_RESET_INSET};
use crate::ui::ops::draw_row_reset;
use crate::ui::rows::{draw_cell, draw_generator_row, draw_item_row, draw_orphan_row};
use crate::ui::state::{Derived, EditorState};
use crate::ui::widgets::{
    cb_state_for, col_at, draw_checkbox, row_frame, selectable_row_keys, toggle_select_all, Cols,
};
use crate::ui_kit::c;

/// Editable columns of a category (from the configuration — no more
/// hard-coded match by category in the code).
pub(crate) fn columns_for(ws: &Workspace, cat: CategoryId) -> Vec<FieldKind> {
    ws.config().category(cat).columns.clone()
}

/// Whether the block belongs to a file in "block = entry" mode (anomalies):
/// every such block is a single anomaly, rendered as one flat, non-collapsible
/// row (no separate generator header, no fold).
pub(crate) fn is_block_entry_block(ws: &Workspace, bid: BlockId) -> bool {
    let roles = &ws.config().parser.block_entry_roles;
    if roles.is_empty() {
        return false;
    }
    let role = &ws.files()[ws.block(bid).file].spec.role;
    roles.iter().any(|r| r == role)
}

/// Inheritance of the "block = entry" files (anomalies, weapons): for each
/// child block whose `refkey` names another block of the same file, map
/// child → parent and parent → [children]. Returns `(parent_of, children_of)`.
/// Only children whose parent is in the same file are tracked (the real
/// anomaly prototypes all live in a single file).
///
/// Depth: the anomaly data is at most 2 levels (base → variant), but the
/// weapon GeneralSetup goes 3 deep (TemplateWeapon → family template →
/// gun) — `render_table` walks this map recursively, at any depth.
pub(crate) fn anomaly_hierarchy(
    ws: &Workspace,
) -> (HashMap<BlockId, BlockId>, HashMap<BlockId, Vec<BlockId>>) {
    let mut parent = HashMap::new();
    let mut children: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
    // Group the block-entry (anomaly) blocks by file: a child's `refkey`
    // names its parent within the SAME file.
    let mut by_file: HashMap<usize, Vec<BlockId>> = HashMap::new();
    for (i, _b) in ws.blocks().iter().enumerate() {
        let bid = BlockId(i as u32);
        if is_block_entry_block(ws, bid) {
            by_file.entry(ws.block(bid).file).or_default().push(bid);
        }
    }
    for bids in by_file.values() {
        let mut by_name: HashMap<String, BlockId> = HashMap::new();
        for &bid in bids {
            by_name.insert(ws.block(bid).name.clone(), bid);
        }
        for &bid in bids {
            // `refkey=Empty` is the technical base: such a block is a top-level
            // root, NOT a child of `Empty`. Only a refkey naming a real anomaly
            // (another in-file block) makes it a child.
            if let Some(refkey) = ws.block(bid).refkey.clone().filter(|r| *r != "Empty") {
                if let Some(&pid) = by_name.get(&refkey) {
                    parent.insert(bid, pid);
                    children.entry(pid).or_default().push(bid);
                }
            }
        }
    }
    for v in children.values_mut() {
        v.sort();
    }
    (parent, children)
}

/// Effective icon key of a block-entry row: the block's own name, or —
/// when it has no icon of its own — the name of the CLOSEST ancestor
/// that does (a weapon variant reuses its gun's icon, whatever the depth
/// of the chain). `None`: neither the block nor any of its ancestors has
/// a real icon (anomaly variants — the row keeps the arrow + tint only).
fn block_icon_key(
    ws: &Workspace,
    bid: BlockId,
    parent: &HashMap<BlockId, BlockId>,
) -> Option<String> {
    let name = ws.block(bid).name.clone();
    if !icon_for(&name).is_placeholder {
        return Some(name);
    }
    parent
        .get(&bid)
        .and_then(|&p| block_icon_key(ws, p, parent))
}

/// Draws a block-entry row and, recursively, its visible children (see
/// `anomaly_hierarchy`). Every child carries ONE "↳" marker, indented by
/// its depth (`depth × HIER_INDENT`, see `draw_item_row`). Children are
/// always shown (no fold); the whole subtree shares the tint assigned by
/// `compute_derived`.
#[allow(clippy::too_many_arguments)] // same profile as the other draw_*_row
fn draw_block_entry_tree(
    ui: &mut Ui,
    ws: &mut Workspace,
    st: &mut EditorState,
    derived: &Derived,
    id: EntryId,
    cols: &[FieldKind],
    geo: &Cols,
    row_w: f32,
    parent: &HashMap<BlockId, BlockId>,
    children: &HashMap<BlockId, Vec<BlockId>>,
    visible: &HashMap<BlockId, EntryId>,
    depth: usize,
) {
    let bid = ws.entry(id).block;
    let row_tint = derived
        .delegation_tint
        .get(&id)
        .copied()
        .or_else(|| derived.block_tint.get(&bid).copied());
    // A child row shows its own icon, or the closest ancestor's (see
    // `block_icon_key`); depth-0 rows (roots, and orphan children promoted
    // by the search) keep the placeholder when the SID has no icon.
    let icon_key = block_icon_key(ws, bid, parent);
    draw_item_row(
        ui,
        ws,
        st,
        id,
        cols,
        geo,
        row_w,
        &derived.visible_idx,
        row_tint,
        depth,
        icon_key.as_deref(),
    );
    let kids: &[BlockId] = children.get(&bid).map_or(&[], Vec::as_slice);
    for &cbid in kids {
        if let Some(&cid) = visible.get(&cbid) {
            draw_block_entry_tree(
                ui,
                ws,
                st,
                derived,
                cid,
                cols,
                geo,
                row_w,
                parent,
                children,
                visible,
                depth + 1,
            );
        }
    }
}

pub(crate) fn render_table(
    ui: &mut Ui,
    ws: &mut Workspace,
    st: &mut EditorState,
    derived: &Derived,
    cols: &[FieldKind],
    geo: &Cols,
) {
    // Inside the scroll area, `available_width` is the viewport:
    // the rows are as wide as the content (they can exceed the viewport,
    // which makes the table scroll horizontally).
    let row_w = ui.available_width().max(geo.content_w);
    let mut last_block: Option<BlockId> = None;
    let mut last_group: Option<FieldId> = None;
    let mut last_group_label: Option<String> = None;

    // Anomaly inheritance: child → parent and parent → [children], plus the
    // entry of every currently visible anomaly block. Children are drawn
    // indented under their parent (when expanded), so they are skipped in
    // the main loop; the parent row draws them.
    let (anom_parent, anom_children) = anomaly_hierarchy(ws);
    let mut visible_anom: HashMap<BlockId, EntryId> = HashMap::new();
    for &vid in &derived.visible_idx {
        let b = ws.entry(vid).block;
        if is_block_entry_block(ws, b) {
            visible_anom.insert(b, vid);
        }
    }

    for &id in &derived.visible_idx {
        let e = &ws.entry(id);
        let bid = e.block;
        // Anomaly-style blocks ("block = entry") render as a single flat row.
        let be = is_block_entry_block(ws, bid);
        let block_entries: &[EntryId] = derived.block_entries.get(&bid).map_or(&[], Vec::as_slice);
        if last_block != Some(bid) {
            // ---- generator row ----
            // Skipped for block-entries: the item row below IS the anomaly
            // (a redundant, foldable header would just repeat its name).
            if be {
            } else {
                draw_generator_row(
                    ui,
                    ws,
                    st,
                    bid,
                    block_entries,
                    row_w,
                    derived.block_tint.get(&bid).copied(),
                );
            }
            last_block = Some(bid);
            last_group = None;
            last_group_label = None;
        }
        // Folded block (default): hide ALL the lines of the block
        // (before, the `continue` above only skipped the 1st entry, the
        // following ones remained). Block-entries are never folded: their
        // rows are always shown.
        if !be && !st.view.expanded.contains(&bid) {
            continue;
        }

        // Block-entry hierarchy (anomalies, weapons): a child is drawn under
        // its parent (handled there) and is skipped here — UNLESS the parent
        // is not visible (e.g. the search only matches the child): in that
        // case the child is drawn here, at the top level, or it would vanish.
        // A root is drawn as a flat item row, immediately followed by its
        // visible children, recursively (always shown — no fold). The whole
        // subtree shares one tint (color-coded).
        if be {
            let parent_visible = anom_parent
                .get(&bid)
                .is_some_and(|&pid| visible_anom.contains_key(&pid));
            if parent_visible {
                continue;
            }
            draw_block_entry_tree(
                ui,
                ws,
                st,
                derived,
                id,
                cols,
                geo,
                row_w,
                &anom_parent,
                &anom_children,
                &visible_anom,
                0,
            );
            continue;
        }

        // Row tint: delegation → tint of its relation; otherwise
        // tint of the block (source/target of a visible delegation).
        let row_tint = derived
            .delegation_tint
            .get(&id)
            .copied()
            .or_else(|| derived.block_tint.get(&bid).copied());
        // ---- group header / group row ----
        let skip_item = draw_group_rows(
            ui,
            ws,
            st,
            id,
            cols,
            geo,
            row_w,
            block_entries,
            &mut last_group,
            &mut last_group_label,
            derived.block_tint.get(&bid).copied(),
        );
        if skip_item {
            continue;
        }

        // ---- item row ----
        draw_item_row(
            ui,
            ws,
            st,
            id,
            cols,
            geo,
            row_w,
            &derived.visible_idx,
            row_tint,
            0,
            None,
        );
    }

    // ---- orphan rows ----
    let search = st.view.search.trim().to_lowercase();
    for o in ws.orphans() {
        if !search.is_empty() && !o.target.to_lowercase().contains(&search) {
            continue;
        }
        let exists = o.entries.iter().any(|&id| {
            let e = ws.entry(id);
            ws.in_category(id, st.view.category) && st.view.file_filter.is_none_or(|f| f == e.file)
        });
        if !exists {
            continue;
        }
        draw_orphan_row(ui, st, &o.target, o.entries.len(), row_w);
    }
}

/// Group lines of an entry (rank, then set/caliber), drawn
/// only if the group has at least one visible cell (stashes:
/// Min/MaxSpawnChance; weapons: MainAmmo if a dedicated column is
/// added). Two levels:
/// - rank (group label, e.g. "Newbie"): one line per rank,
///   only when the group carries itemsets;
/// - set / caliber: one line per group field key — the spawn
///   chances are carried by the set, not by the rank.
///
/// `last_group` / `last_group_label` are the rendering state of the loop.
/// Returns `true` if the item row must be skipped (rank sub-dropdown
/// folded: sets + items hidden).
#[allow(clippy::too_many_arguments)] // same profile as the other draw_*_row
pub(crate) fn draw_group_rows(
    ui: &mut Ui,
    ws: &mut Workspace,
    st: &mut EditorState,
    id: EntryId,
    cols: &[FieldKind],
    geo: &Cols,
    row_w: f32,
    block_entries: &[EntryId],
    last_group: &mut Option<FieldId>,
    last_group_label: &mut Option<String>,
    tint: Option<Color32>,
) -> bool {
    // `gkeys` is copied before the calls that take `ws` mutably
    // (cells, reset): `e` is no longer borrowed at that point (NLL).
    let e = ws.entry(id);
    let gkeys = e.group_fields.clone();
    // ---- group header / group row ----
    if !gkeys.is_empty() && gkeys.iter().any(|&k| cols.contains(&ws.field(k).kind)) {
        let g = gkeys[0];
        let glabel = e.group.clone().unwrap_or_else(|| "group".into());
        // Rank = collapsible sub-dropdown: (block, label of the rank).
        let rkey: (BlockId, String) = (e.block, glabel.clone());
        if e.itemset_label.is_some() && last_group_label.as_deref() != Some(glabel.as_str()) {
            let rl = RowLayout::rank();
            let rect = row_frame(ui, row_w, rl.h, None, tint);
            let painter = ui.painter_at(rect);
            // Triangle of the sub-dropdown (more indented than that of the
            // block) + name: geometry from the shared rank row layout.
            let rcollapsed = st.view.rank_collapsed.contains(&rkey);
            rl.paint_tri(&painter, rect, rcollapsed);
            painter.text(
                rl.name_pos(rect),
                Align2::LEFT_CENTER,
                glabel.clone(),
                FontId::proportional(FONT_BODY),
                c::TEXT_STRONG,
            );
            // "Select all the rank" checkbox (same column as the
            // item rows and the blocks).
            let rcb_keys = selectable_row_keys(ws, block_entries, Some(glabel.as_str()));
            if let Some(cb) = rl.cb_rect(rect) {
                draw_checkbox(
                    &painter,
                    cb.center(),
                    cb_state_for(&rcb_keys, &st.selection.batch),
                );
            }
            if ui
                .interact(
                    rect,
                    Id::new(("rank", e.block.idx(), glabel.as_str())),
                    Sense::click(),
                )
                .clicked()
            {
                // Zone of the triangle (to the right of the checkbox): fold.
                let near_tri = ui.input(|i| {
                    i.pointer
                        .interact_pos()
                        .is_some_and(|p| rl.tri_hit(rect, p.x))
                });
                if near_tri {
                    if st.view.rank_collapsed.contains(&rkey) {
                        st.view.rank_collapsed.remove(&rkey);
                    } else {
                        st.view.rank_collapsed.insert(rkey.clone());
                    }
                }
            }
            if let Some(rankcb) = rl.cb_rect(rect) {
                if ui
                    .interact(
                        rankcb,
                        Id::new(("rankcb", e.block.idx(), glabel.as_str())),
                        Sense::click(),
                    )
                    .clicked()
                {
                    let keys = selectable_row_keys(ws, block_entries, Some(glabel.as_str()));
                    toggle_select_all(st, &keys);
                }
            }
            *last_group = None;
            *last_group_label = Some(glabel.clone());
        }
        // Folded rank sub-dropdown: hide the sets + items of this rank.
        if e.itemset_label.is_some() && st.view.rank_collapsed.contains(&rkey) {
            return true;
        }
        if *last_group != Some(g) {
            let srl = RowLayout::set_row();
            let rect = row_frame(ui, row_w, srl.h, None, tint);
            let painter = ui.painter_at(rect);
            if let Some(isl) = &e.itemset_label {
                if let Some(p) = srl.set_label_pos(rect) {
                    painter.text(
                        p,
                        Align2::LEFT_CENTER,
                        isl.clone(),
                        FontId::proportional(FONT_BUTTON),
                        c::TEXT_DIM,
                    );
                }
            } else {
                painter.text(
                    srl.name_pos(rect),
                    Align2::LEFT_CENTER,
                    glabel.as_str(),
                    FontId::proportional(FONT_BODY),
                    c::TEXT,
                );
            }
            for (ci, col) in cols.iter().enumerate() {
                if let Some(&gk) = gkeys.iter().find(|&&k| ws.field(k).kind == *col) {
                    draw_cell(
                        ui,
                        ws,
                        st,
                        gk,
                        &col_at(&geo.cols[ci], rect.left()),
                        rect.center().y,
                    );
                }
            }
            // Reset button of the group line (visible if a
            // field of the group has been modified).
            draw_row_reset(
                ui,
                ws,
                st,
                &gkeys,
                pos2(rect.right() - ROW_RESET_INSET, rect.center().y),
            );
            *last_group = Some(g);
        }
    } else {
        *last_group = None;
        *last_group_label = None;
    }
    false
}
