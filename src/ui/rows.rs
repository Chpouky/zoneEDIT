//! Table rows: generator, item (delegation included), orphan, cell.

use crate::ui::icons::{icon_for, ItemIcon};
use egui::{
    pos2, vec2, Align, Align2, Color32, CornerRadius, FontId, Id, Layout, Rect, Sense, Stroke,
    StrokeKind, Ui,
};

use crate::domain::{value, BlockId, EntryId, FieldId, FieldKind, Workspace};
use crate::ui::layout::{paint_block_label, BlockLabel, RowLayout};
use crate::ui::metrics::{
    CELL_BOX_H, CELL_BOX_W, CELL_FONT, CELL_RADIUS, FONT_BODY, FONT_SMALL, HIER_INDENT, ICON_BOX_W,
    ICON_H, ICON_X, LINE_FONT, LINE_X_GAP, ORPHAN_TRI_X, RANK_LABEL_GAP, ROW_H_TALL,
    ROW_RESET_INSET, TAG_H, TAG_TOP, TAG_W, TAG_X_GAP,
};
use crate::ui::ops::{commit, delegation_target, draw_row_reset};
use crate::ui::state::{entry_rank, EditorState, Modal};
use crate::ui::widgets::{
    btn, cb_state_for, col_at, draw_checkbox, fit_label, row_frame, selectable_row_keys,
    toggle_select_all, CbState, Col, Cols,
};
use crate::ui_kit::{c, tag};

/// Draws the "generator" row (block header): "select all"
/// checkbox, fold/unfold triangle, block name + line,
/// delegation references tag.
pub(crate) fn draw_generator_row(
    ui: &mut Ui,
    ws: &Workspace,
    st: &mut EditorState,
    bid: BlockId,
    block_entries: &[EntryId],
    row_w: f32,
    tint: Option<Color32>,
) {
    let rl = RowLayout::generator();
    // Generator header rows are painted over the panel background.
    let rect = row_frame(ui, row_w, rl.h, Some(c::PANEL), tint);
    {
        let painter = ui.painter_at(rect);
        let collapsed = !st.view.expanded.contains(&bid);
        // "Select whole block" checkbox — column common to all
        // levels; triangle and name shifted to its right.
        let cb_keys = selectable_row_keys(ws, block_entries, None);
        if let Some(cb) = rl.cb_rect(rect) {
            draw_checkbox(
                &painter,
                cb.center(),
                cb_state_for(&cb_keys, &st.selection.batch),
            );
        }
        rl.paint_tri(&painter, rect, collapsed);
        let b = ws.block(bid);
        paint_block_label(
            &painter,
            ui,
            rect,
            &BlockLabel {
                name_x: rl.name_x,
                name: &b.name,
                line: b.line,
                refs: b.delegation_refs,
                show_refs: st.view.show_refs,
            },
        );
    }
    if ui
        .interact(rect, Id::new(("gen", bid.idx())), Sense::click())
        .clicked()
    {
        // Triangle zone (right of the checkbox): fold/unfold.
        let near_tri = ui.input(|i| {
            i.pointer
                .interact_pos()
                .is_some_and(|p| rl.tri_hit(rect, p.x))
        });
        if near_tri {
            if st.view.expanded.contains(&bid) {
                st.view.expanded.remove(&bid);
            } else {
                st.view.expanded.insert(bid);
            }
        }
    }
    // "Select whole block" checkbox: registered after the rect of the
    // whole block to catch the clicks of its zone.
    if let Some(gencb) = rl.cb_rect(rect) {
        if ui
            .interact(gencb, Id::new(("gencb", bid.idx())), Sense::click())
            .clicked()
        {
            let keys = selectable_row_keys(ws, block_entries, None);
            toggle_select_all(st, &keys);
        }
    }
}

/// Trader generator row (TRADERS page): "select all" checkbox + block
/// name (strong) + line + refs tag. No fold triangle — the items of the
/// generator are always drawn beneath it, indented with the "↳" marker
/// (same grouping model as the anomaly child rows, without the fold).
pub(crate) fn draw_trade_parent_row(
    ui: &mut Ui,
    ws: &Workspace,
    st: &mut EditorState,
    bid: BlockId,
    block_entries: &[EntryId],
    row_w: f32,
    tint: Option<Color32>,
) {
    let rl = RowLayout::trade();
    // Trader header rows are painted over the panel background.
    let rect = row_frame(ui, row_w, rl.h, Some(c::PANEL), tint);
    {
        let painter = ui.painter_at(rect);
        // "Select whole block" checkbox — column common to all levels.
        let cb_keys = selectable_row_keys(ws, block_entries, None);
        if let Some(cb) = rl.cb_rect(rect) {
            draw_checkbox(
                &painter,
                cb.center(),
                cb_state_for(&cb_keys, &st.selection.batch),
            );
        }
        let b = ws.block(bid);
        paint_block_label(
            &painter,
            ui,
            rect,
            &BlockLabel {
                name_x: rl.name_x,
                name: &b.name,
                line: b.line,
                refs: b.delegation_refs,
                show_refs: st.view.show_refs,
            },
        );
    }
    // Click on the row: select all (the row is never folded, so there is
    // no triangle zone to test).
    if ui
        .interact(rect, Id::new(("trade", bid.idx())), Sense::click())
        .clicked()
    {
        let keys = selectable_row_keys(ws, block_entries, None);
        toggle_select_all(st, &keys);
    }
    // "Select whole block" checkbox: registered after the rect of the whole
    // block to catch the clicks of its zone.
    if let Some(trcb) = rl.cb_rect(rect) {
        if ui
            .interact(trcb, Id::new(("tradecb", bid.idx())), Sense::click())
            .clicked()
        {
            let keys = selectable_row_keys(ws, block_entries, None);
            toggle_select_all(st, &keys);
        }
    }
}

/// The icon of a row (item column): scaled into the icon box + the
/// "icon at original size" pop-up on hover (cursor following).
fn paint_row_icon(ui: &mut Ui, rect: Rect, icon: &ItemIcon) {
    let rl = RowLayout::item();
    let scale = (ICON_H / icon.size.y).min(ICON_BOX_W / icon.size.x);
    let icon_size = icon.size * scale;
    if let Some(icon_rect) = rl.icon_rect(rect, icon_size) {
        egui::Image::new(icon.src.clone())
            .fit_to_exact_size(icon_size)
            .paint_at(ui, icon_rect);
        let hovered = ui
            .ctx()
            .input(|i| i.pointer.hover_pos().filter(|_| !i.pointer.any_down()));
        if let Some(pos) = hovered.filter(|pos| icon_rect.contains(*pos)) {
            crate::ui::icons::draw_tooltip(ui, icon, pos);
        }
    }
}

/// Draws an item row (regular row or delegation): checkbox, icon, name,
/// value cells, row reset button, selection (click / Ctrl /
/// Shift). `visible` is the list of displayed entries (screen order), for
/// the Shift+click selection extension.
// Faithful port of the old signature + `visible` (Shift+click extension):
// 8 parameters, explicitly tolerated.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_item_row(
    ui: &mut Ui,
    ws: &mut Workspace,
    st: &mut EditorState,
    id: EntryId,
    cols: &[FieldKind],
    geo: &Cols,
    row_w: f32,
    visible: &[EntryId],
    tint: Option<Color32>,
    // Depth in the block-entry hierarchy (0 = regular/root row, 1 = child,
    // 2 = grandchild — weapons are 3 levels deep: TemplateWeapon → family
    // template → gun). Every child carries ONE "↳" marker; the depth is
    // shown by the name indent (`depth × HIER_INDENT`), not by arrows.
    child_depth: usize,
    // Block-entry rows only: the icon key to paint when the block has no
    // icon of its own (the closest ancestor's — see `block_icon_key` in
    // `table.rs`). `None` outside block-entry hierarchies.
    icon_key_override: Option<&str>,
) {
    // `fids` is copied before the calls that take `ws` mutably
    // (cells, reset): `e` is no longer borrowed at that point (NLL).
    let e = &ws.entry(id);
    let fids = e.fields.clone();
    // Left-side layout: a regular item row. An anomaly PARENT is a plain item
    // row; an anomaly CHILD uses the same layout (same name column) — the only
    // difference is the "↳" prefix on its name, exactly like a delegation row.
    let rl = RowLayout::item();
    // ---- item row ----
    let sel = !fids.is_empty();
    // Unique identifier per entry (`EntryId` is already unique per
    // occurrence: the same item can exist in several ranks).
    let row_id = Id::new(("row", id.idx()));
    let row_checked = st.selection.batch.iter().any(|k| fids.contains(k));
    // Selected rows are painted over the hover background.
    let rect = row_frame(ui, row_w, rl.h, row_checked.then_some(c::HOVER), tint);
    let painter = ui.painter_at(rect);
    let cy = rect.center().y;
    if sel {
        if let Some(cb) = rl.cb_rect(rect) {
            draw_checkbox(
                &painter,
                cb.center(),
                if row_checked {
                    CbState::Checked
                } else {
                    CbState::Off
                },
            );
        }
    }
    // Icon: regular (depth-0) item rows always get one — the placeholder
    // when the SID has no icon file (world spawner rows use the icon of
    // their first listed artifact, the `icon_sid` hint). Hierarchy CHILD
    // rows get an icon only when a REAL one exists: their own first (a
    // trade-block item keeps its gun/armor/… icon), then the inherited
    // one from the closest ancestor with one (`icon_key_override` — a
    // weapon variant reuses its gun's icon). A child without any real
    // icon stays icon-less: the "↳" arrow and the tint already group it
    // with its parent (anomaly variants have no icon file).
    if child_depth == 0 {
        paint_row_icon(ui, rect, &icon_for(e.icon_key()));
    } else {
        let own = icon_for(e.icon_key());
        if !own.is_placeholder {
            paint_row_icon(ui, rect, &own);
        } else if let Some(key) = icon_key_override {
            let icon = icon_for(key);
            if !icon.is_placeholder {
                paint_row_icon(ui, rect, &icon);
            }
        }
    }
    // name — truncated so it never slides under the first value column
    // (e.g. long anomaly SIDs on the 11-column anomalies page).
    let name_x = rl.name_pos(rect).x;
    // The `geo` centers are body-local: the row's left edge carries the
    // scroll offsets (the name zone scrolls with the row, not under it).
    let name_max_w = match geo.cols.first() {
        Some(c0) => rect.left() + c0.center - c0.width / 2.0 - name_x - 8.0,
        None => f32::INFINITY,
    };
    // name
    if e.is_delegation() {
        let t = delegation_target(e);
        let painter = ui.painter_at(rect);
        let font = FontId::proportional(FONT_BODY);
        // Hierarchy child (trade block): the "↳ target" label carries the
        // same indent as a regular child name.
        let dx = child_depth as f32 * HIER_INDENT;
        let label = fit_label(&painter, &format!("↳ {t}"), &font, name_max_w - dx);
        painter.text(
            pos2(name_x + dx, cy),
            Align2::LEFT_CENTER,
            label,
            font,
            c::TEXT_DIM,
        );
    } else {
        let painter = ui.painter_at(rect);
        let font = FontId::proportional(FONT_BODY);
        // Hierarchy child: ONE "↳" marker (always, whatever the depth) and
        // the name shifted right by `child_depth × HIER_INDENT` — the
        // offset, not a stack of arrows, encodes the depth (same model as a
        // delegation row: arrow + tint group the child with its parent).
        // No icon, no separate arrow. The checkbox and the value cells stay
        // anchored: only the name zone moves.
        let name_x = rl.name_pos(rect).x + child_depth as f32 * HIER_INDENT;
        let name = if child_depth > 0 {
            fit_label(
                &painter,
                &format!("↳ {}", e.sid),
                &font,
                name_max_w - child_depth as f32 * HIER_INDENT,
            )
        } else {
            fit_label(&painter, &e.sid, &font, name_max_w)
        };
        let name_w = painter
            .text(pos2(name_x, cy), Align2::LEFT_CENTER, name, font, c::TEXT)
            .width();
        // Small rank label after the name, to distinguish the tables
        // of the same block (Newbie / Experienced / …). Always displayed,
        // even when a specific rank is filtered.
        if let Some(rank) = entry_rank(&e.ctx) {
            painter.text(
                pos2(name_x + name_w + RANK_LABEL_GAP, cy),
                Align2::LEFT_CENTER,
                rank,
                FontId::proportional(FONT_SMALL),
                c::TEXT_MUTED,
            );
        }
    }
    // selection — registered BEFORE the cells: in egui, the last
    // interact on the same rect wins, so the cells (and the editor)
    // must be registered after to capture their clicks.
    let mut row_clicked = false;
    let mut cb_clicked = false;
    if sel {
        if ui.interact(rect, row_id, Sense::click()).clicked() {
            row_clicked = true;
        }
        // "Select the row" checkbox: registered after the rect of the
        // row to catch the clicks of its zone.
        if let Some(rowcb) = rl.cb_rect(rect) {
            if ui
                .interact(rowcb, Id::new(("rowcb", id.idx())), Sense::click())
                .clicked()
            {
                cb_clicked = true;
            }
        }
    }
    // value cells
    for (ci, col) in cols.iter().enumerate() {
        if let Some(&f) = fids.iter().find(|&&f| ws.field(f).kind == *col) {
            draw_cell(ui, ws, st, f, &col_at(&geo.cols[ci], rect.left()), cy);
        }
    }
    // Small circular row-reset button, visible only if one of the row's
    // values changed.
    let row_reset_clicked =
        draw_row_reset(ui, ws, st, &fids, pos2(rect.right() - ROW_RESET_INSET, cy));
    // Checkbox: simple toggle of the row (no modifiers).
    if cb_clicked && st.editing.cell.is_none() {
        if let Some(first) = fids.first().copied() {
            if st.selection.batch.contains(&first) {
                st.selection.batch.remove(&first);
            } else {
                st.selection.batch.insert(first);
            }
            st.selection.last_clicked = Some(id);
        }
    }
    // selection: applied after the cells (a clicked cell opens
    // the editor; the row selection must not happen at the same time)
    if row_clicked && !row_reset_clicked && st.editing.cell.is_none() {
        if let Some(first) = fids.first().copied() {
            let ctrl = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
            let mut shift = ui.input(|i| i.modifiers.shift);
            if ctrl {
                if st.selection.batch.contains(&first) {
                    st.selection.batch.remove(&first);
                } else {
                    st.selection.batch.insert(first);
                }
                st.selection.last_clicked = Some(id);
            } else if shift {
                if let Some(last) = st.selection.last_clicked {
                    if !st.selection.batch.contains(&first) {
                        st.selection.batch.insert(first);
                    }
                    // Bug fixed (v5): the range is computed in DISPLAY
                    // order (the `visible` list), not in selection
                    // order as in the old code.
                    if let (Some(a), Some(b)) = (
                        visible.iter().position(|&v| v == id),
                        visible.iter().position(|&v| v == last),
                    ) {
                        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
                        for &v in &visible[lo..=hi] {
                            if let Some(fid) = ws.entry(v).fields.first() {
                                st.selection.batch.insert(*fid);
                            }
                        }
                    }
                    shift = false;
                }
                if shift {
                    st.selection.batch.clear();
                    st.selection.batch.insert(first);
                }
                st.selection.last_clicked = Some(id);
            } else {
                st.selection.batch.clear();
                st.selection.batch.insert(first);
                st.selection.last_clicked = Some(id);
            }
        }
    }
}

/// Draws an orphan row (unfound delegation target):
/// red triangle, name, "N refs" tag, "Redirect…" button.
pub(crate) fn draw_orphan_row(
    ui: &mut Ui,
    st: &mut EditorState,
    target: &str,
    nrefs: usize,
    row_w: f32,
) {
    let rect = row_frame(ui, row_w, ROW_H_TALL, None, None);
    let cy = rect.center().y;
    {
        let painter = ui.painter_at(rect);
        // Warning triangle (not a fold triangle): same slot as the item icon.
        let tri = [
            pos2(rect.left() + ORPHAN_TRI_X, cy - 4.5),
            pos2(rect.left() + ORPHAN_TRI_X, cy + 4.5),
            pos2(rect.left() + ORPHAN_TRI_X + 5.5, cy),
        ];
        painter.add(egui::Shape::convex_polygon(
            tri.to_vec(),
            c::DANGER_FG,
            Stroke::NONE,
        ));
        let name_w = painter
            .text(
                pos2(rect.left() + ICON_X, cy),
                Align2::LEFT_CENTER,
                target,
                FontId::proportional(FONT_BODY),
                c::TEXT_STRONG,
            )
            .width();
        let nx = rect.left() + ICON_X + name_w + LINE_X_GAP;
        let nf_w = painter
            .text(
                pos2(nx, cy),
                Align2::LEFT_CENTER,
                "not found",
                FontId::monospace(LINE_FONT),
                c::TEXT_MUTED,
            )
            .width();
        let area = Rect::from_min_size(
            pos2(nx + nf_w + TAG_X_GAP, rect.top() + TAG_TOP),
            vec2(TAG_W, TAG_H),
        );
        let mut ch = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(area)
                .layout(Layout::left_to_right(Align::Center)),
        );
        tag(&mut ch, &format!("orphan · {nrefs} refs"), true);
    }
    let btn_area = Rect::from_center_size(pos2(rect.right() - 16.0 - 45.0, cy), vec2(90.0, 26.0));
    let mut ch = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(btn_area)
            .layout(Layout::left_to_right(Align::Center)),
    );
    if btn(&mut ch, "Redirect…").clicked() {
        st.modal = Some(Modal::Redirect(target.to_string()));
    }
}

pub(crate) fn draw_cell(
    ui: &mut Ui,
    ws: &mut Workspace,
    st: &mut EditorState,
    fid: FieldId,
    col: &Col,
    center_y: f32,
) {
    let (start, invalid, is_dirty) = {
        let f = ws.field(fid);
        let raw = f.value().to_string();
        // Spawn chances: the UI displays/edits them in percent.
        let spec = ws.spec(f.kind);
        let start = value::display(spec, &raw);
        (start, !f.is_valid(), f.is_dirty())
    };
    // The box is as wide as the cell of the column, with a small margin.
    let box_w = CELL_BOX_W.min(col.width - 8.0);
    let rect = Rect::from_center_size(pos2(col.center, center_y), vec2(box_w, CELL_BOX_H));
    if st.editing.cell == Some(fid) {
        let mut text = st.editing.text.clone();
        let mut ch = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(Layout::left_to_right(Align::Center)),
        );
        let id = egui::Id::new(fid);
        // 1st frame of the session: focus is requested BEFORE the creation of the
        // TextEdit. Without this, `has_focus` is false during the layout and the
        // first keystroke of the frame is swallowed (eframe only repaints on
        // event: the opening and the first keystroke often arrive on
        // the same frame). The full selection is pre-seeded so that the keystroke
        // replaces the value (egui 0.31 has no `.selection()` builder);
        // a re-seed per frame would block cursor movement.
        let first_frame = st.editing.drawn_gen != st.editing.gen;
        if first_frame {
            ch.ctx().memory_mut(|m| m.request_focus(id));
            ch.ctx().data_mut(|d| {
                let te =
                    d.get_persisted_mut_or_default::<egui::widgets::text_edit::TextEditState>(id);
                te.cursor =
                    egui::text_selection::TextCursorState::from(egui::text::CCursorRange::two(
                        egui::text::CCursor::new(0),
                        egui::text::CCursor::new(text.chars().count()),
                    ));
            });
        }
        let resp = ch.add(
            egui::TextEdit::singleline(&mut text)
                .id(id)
                .font(egui::TextStyle::Monospace)
                .desired_width(box_w)
                .frame(false),
        );
        st.editing.text = text;
        // Commit only on focus loss: `resp.changed()` would be true from the
        // very first key (the editor would close after 1 character).
        // In singleline, Enter loses the focus, so it is covered.
        if resp.lost_focus() {
            let v = st.editing.text.trim().to_string();
            match commit(ws, fid, &v) {
                Ok(()) => {}
                Err(m) => st.msg = Some((false, m)),
            }
            st.editing.cell = None;
        } else if !ch.ctx().memory(|m| m.has_focus(resp.id)) {
            resp.request_focus();
        }
        st.editing.drawn_gen = st.editing.gen;
    } else {
        let (fill, fg, stroke) = if invalid {
            (c::DANGER_BG, c::DANGER_FG, c::DANGER_FG)
        } else if is_dirty {
            (c::MODIFIED_BG, c::MODIFIED_FG, c::MODIFIED_FG)
        } else {
            (c::INPUT_BG, c::TEXT, c::BORDER_CTRL)
        };
        let mut ch = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        {
            let (rect2, resp) = ch.allocate_exact_size(vec2(box_w, CELL_BOX_H), Sense::click());
            if resp.clicked() {
                st.editing.cell = Some(fid);
                st.editing.gen = st.editing.gen.wrapping_add(1);
                st.editing.text = start.clone();
            }
            let p = ch.painter_at(rect2);
            p.rect_filled(rect2, CornerRadius::same(CELL_RADIUS), fill);
            p.rect_stroke(
                rect2,
                CornerRadius::same(CELL_RADIUS),
                Stroke::new(1.0_f32, stroke),
                StrokeKind::Inside,
            );
            p.text(
                rect2.center(),
                Align2::CENTER_CENTER,
                start,
                FontId::monospace(CELL_FONT),
                fg,
            );
        }
    }
}
