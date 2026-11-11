//! selection — selection by checkbox + batch actions + errors — headless UI tests (egui without GPU).

use crate::domain::{BlockId, EntryId, FieldId, FieldKind};
use crate::ui::ops::apply_action;
use crate::ui::*;
use egui::{pos2, Pos2};

use super::common::*;
// ---- Actions (applied in batch at the end of the frame) ----------------------
#[test]
fn actions_are_batched_applied_after_take() {
    let mut ws = test_workspace();
    let mut st = EditorState::default();

    // Dirties a counter field (entirely guaranteed valid and different
    // from the original — a blind commit on a random HashMap key
    // would be non deterministic), then ResetAll via the actions queue
    // (no selection: ResetSelection would do nothing).
    let key = ws
        .field_ids()
        .find(|&k| matches!(ws.field(k).kind, FieldKind::MinCount | FieldKind::MaxCount))
        .expect("a counter field in the fixtures");
    {
        let v = if ws.field(key).original() == "0" {
            "1"
        } else {
            "0"
        };
        ws.set_text(key, v);
    }
    assert!(ws.dirty_count() > 0, "field dirtied for the test");
    st.push_action(Action::ResetAll);
    for a in st.take_actions() {
        apply_action(&mut ws, &mut st, a);
    }
    assert_eq!(ws.dirty_count(), 0, "ResetAll must reset the fields");
    assert!(st.take_actions().is_empty(), "empty queue after take");

    // ShowDiff then ShowErrors set the corresponding modal.
    apply_action(&mut ws, &mut st, Action::ShowDiff);
    assert!(matches!(st.modal, Some(Modal::Diff)));
    apply_action(&mut ws, &mut st, Action::ShowErrors);
    assert!(matches!(st.modal, Some(Modal::Errors)));
}

// ---- Derived state (step 3.8): grouping by block --------------------
#[test]
fn derived_block_groups_drive_select_all() {
    let ws = test_workspace();
    let st = EditorState::default();
    let d = compute_derived(&ws, &st);

    // The indices grouped by block exactly reconstitute the set
    // of visible entries (no entry lost nor duplicated).
    let total: usize = d.block_entries.values().map(Vec::len).sum();
    assert_eq!(
        total,
        d.visible_idx.len(),
        "grouping without loss nor duplication"
    );
    for (&bid, idxs) in &d.block_entries {
        assert!(!idxs.is_empty());
        for &id in idxs {
            let e = ws.entry(id);
            assert_eq!(e.block, bid);
        }
    }

    // The "select all" keys of a block = 1st field of each
    // entry of the group (no more scan of the whole set of visible).
    for (idxs, keys) in d
        .block_entries
        .values()
        .map(|idxs| (idxs, selectable_row_keys(&ws, idxs, None)))
    {
        assert_eq!(
            keys.len(),
            idxs.iter()
                .filter(|&&id| !ws.entry(id).fields.is_empty())
                .count()
        );
        for k in &keys {
            assert!(
                idxs.iter().any(|&id| ws.entry(id).fields.contains(k)),
                "key outside the block"
            );
        }
    }
}

/// "N error(s)" in the status bar must be clickable: the click
/// opens a panel listing the details of the validation errors.
#[test]
fn status_bar_error_count_opens_details() {
    let mut ws = test_workspace();
    let mut st = EditorState::default();
    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);

    // A dirty field with an invalid value (negative) → 1 error.
    let entries = visible_entries(&ws, &st);
    let first = entries
        .iter()
        .find(|e| !e.is_delegation() && !e.fields.is_empty())
        .expect("visible entry with fields");
    let key = first.fields[0];
    ws.set_text(key, "-5");
    assert_eq!(
        ws.validation_errors().len(),
        1,
        "an invalid field must produce an error"
    );

    // Frame 1: the error count is displayed in the status bar.
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();
    let label_pos = out
        .shapes
        .iter()
        .find_map(|cs| {
            if let egui::Shape::Text(t) = &cs.shape {
                if t.galley.text() == "1 error(s) ▾" {
                    Some(t.pos)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .expect("\"1 error(s)\" must be displayed in the status bar");

    // Frame 2: click on the count → the panel opens.
    let mut raw = raw_1440();
    let click = pos2(label_pos.x + 10.0, label_pos.y + 8.0);
    raw.events.push(egui::Event::PointerButton {
        pos: click,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    });
    raw.events.push(egui::Event::PointerButton {
        pos: click,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    });
    ctx.begin_pass(raw);
    show_editor(&ctx, &mut ws, &mut st);
    let _ = ctx.end_pass();
    assert!(
        matches!(st.modal, Some(Modal::Errors)),
        "the click on the error count must open the panel"
    );
    // Frame 3: the panel is painted on the frame following the click
    // (the action is applied in batch, at the end of the frame).
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();
    let texts: Vec<&str> = out
        .shapes
        .iter()
        .filter_map(|cs| {
            if let egui::Shape::Text(t) = &cs.shape {
                Some(t.galley.text())
            } else {
                None
            }
        })
        .collect();
    assert!(
        texts.iter().any(|t| t.contains("must be ≥ 0")),
        "the panel must display the details of the error; texts: {texts:?}"
    );
}

/// The checkbox of an item row does a simple toggle of the selection of
/// the row, without modifiers.
#[test]
fn item_row_checkbox_toggles_row_selection() {
    let mut ws = test_workspace();
    let mut st = EditorState::default();
    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);

    // A visible entry with fields, whose item name is unique
    // in its block: only this block is expanded, so the name can
    // appear only once in the table.
    let visible = visible_indices(&ws, &st);
    let mut by_block: std::collections::HashMap<BlockId, Vec<EntryId>> =
        std::collections::HashMap::new();
    for &id in &visible {
        by_block.entry(ws.entry(id).block).or_default().push(id);
    }
    let (bkey, e) = by_block
        .iter()
        .find_map(|(&bk, ids)| {
            ids.iter()
                .find(|&&id| {
                    let e = ws.entry(id);
                    !e.is_delegation() && !e.fields.is_empty()
                })
                .filter(|&&id| {
                    ids.iter()
                        .filter(|&&o| ws.entry(o).sid == ws.entry(id).sid)
                        .count()
                        == 1
                })
                .map(|&id| (bk, id))
        })
        .expect("a block with a unique item name");
    st.view.expanded.insert(bkey);
    let first = ws.entry(e).fields[0];
    let e_sid = ws.entry(e).sid.clone();

    // Frame 1: position of the item name (x = left+78) → checkbox (left+22).
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();
    let name = out
        .shapes
        .iter()
        .filter_map(|cs| {
            if let egui::Shape::Text(t) = &cs.shape {
                if t.galley.text() == e_sid && t.pos.x > 300.0 && t.pos.y > 90.0 {
                    Some((t.pos, t.galley.size().y))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .min_by(|a, b| a.0.y.partial_cmp(&b.0.y).unwrap())
        .expect("item name painted");
    // The checkbox is at `CB_CENTER_X` from the edge of the row; the name at
    // `ITEM_NAME_X` — hence the offset (fixed, whatever the icon).
    let cb = pos2(
        name.0.x - (crate::ui::metrics::ITEM_NAME_X - crate::ui::metrics::CB_CENTER_X),
        name.0.y + name.1 / 2.0,
    );

    // Frame 2: click on the checkbox → the row is selected.
    let mut raw = raw_1440();
    click(&mut raw, cb);
    ctx.begin_pass(raw);
    show_editor(&ctx, &mut ws, &mut st);
    let _ = ctx.end_pass();
    assert_eq!(
        st.selection.batch.len(),
        1,
        "the checkbox click must select the row"
    );
    assert!(
        st.selection.batch.contains(&first),
        "it is the 1st field of the row"
    );

    // Frame 3: a second click → the row is unselected.
    let mut raw = raw_1440();
    click(&mut raw, cb);
    ctx.begin_pass(raw);
    show_editor(&ctx, &mut ws, &mut st);
    let _ = ctx.end_pass();
    assert!(
        st.selection.batch.is_empty(),
        "a second click on the checkbox must unselect the row"
    );
}

/// The checkbox of the block line selects/unselects ALL the
/// rows of the block; in the partial state (a single row), it checks the rest.
#[test]
fn block_checkbox_selects_all_rows_of_block() {
    let mut ws = test_workspace();
    let mut st = EditorState::default();
    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);

    // Visible block with the most entries (>= 2, non delegations).
    let visible = visible_indices(&ws, &st);
    let mut by_block: std::collections::HashMap<BlockId, Vec<EntryId>> =
        std::collections::HashMap::new();
    for &id in &visible {
        by_block.entry(ws.entry(id).block).or_default().push(id);
    }
    let (&bkey, block_entries) = by_block
        .iter()
        .filter(|(_, v)| v.len() >= 2 && v.iter().all(|&id| !ws.entry(id).is_delegation()))
        .max_by_key(|(_, v)| v.len())
        .expect("a block with >= 2 entries");
    let block_name = ws.block(bkey).name.clone();
    let item_names: Vec<String> = block_entries
        .iter()
        .map(|&id| ws.entry(id).sid.clone())
        .collect();
    let expected: Vec<FieldId> = block_entries
        .iter()
        .filter_map(|&id| ws.entry(id).fields.first().copied())
        .collect();
    st.view.expanded.insert(bkey);

    /// Position of the block checkbox, found via the name of the
    /// generatrix (x = left+60 → checkbox left+22).
    fn block_cb(out: &egui::FullOutput, block_name: &str) -> Pos2 {
        let (p, h) = out
            .shapes
            .iter()
            .filter_map(|cs| {
                if let egui::Shape::Text(t) = &cs.shape {
                    if t.galley.text() == block_name && t.pos.x > 300.0 && t.pos.y > 90.0 {
                        Some((t.pos, t.galley.size().y))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .min_by(|a, b| a.0.y.partial_cmp(&b.0.y).unwrap())
            .expect("generatrix name painted");
        pos2(p.x - 38.0, p.y + h / 2.0)
    }

    /// Position of the checkbox of an item row via its name (x = left+78
    /// → checkbox left+22).
    fn row_cb(out: &egui::FullOutput, name: &str, y0: f32) -> Pos2 {
        let (p, h) = out
            .shapes
            .iter()
            .filter_map(|cs| {
                if let egui::Shape::Text(t) = &cs.shape {
                    if t.galley.text() == name && t.pos.x > 325.0 && t.pos.y > y0 {
                        Some((t.pos, t.galley.size().y))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .min_by(|a, b| a.0.y.partial_cmp(&b.0.y).unwrap())
            .expect("item name painted");
        pos2(
            p.x - (crate::ui::metrics::ITEM_NAME_X - crate::ui::metrics::CB_CENTER_X),
            p.y + h / 2.0,
        )
    }

    // Frame 1: locate the block checkbox.
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();
    let gencb = block_cb(&out, &block_name);

    // Frame 2: click → all the rows of the block are selected.
    let mut raw = raw_1440();
    click(&mut raw, gencb);
    ctx.begin_pass(raw);
    show_editor(&ctx, &mut ws, &mut st);
    let _ = ctx.end_pass();
    assert_eq!(
        st.selection.batch.len(),
        expected.len(),
        "the block checkbox must select all its rows"
    );
    for k in &expected {
        assert!(st.selection.batch.contains(k), "block row not selected");
    }

    // Frame 3: click again → everything is unselected.
    let mut raw = raw_1440();
    click(&mut raw, gencb);
    ctx.begin_pass(raw);
    show_editor(&ctx, &mut ws, &mut st);
    let _ = ctx.end_pass();
    assert!(
        st.selection.batch.is_empty(),
        "the second click must unselect everything"
    );

    // Frame 4: select a single row (partial state).
    let mut raw = raw_1440();
    click(&mut raw, row_cb(&out, &item_names[0], 90.0));
    ctx.begin_pass(raw);
    show_editor(&ctx, &mut ws, &mut st);
    let _ = ctx.end_pass();
    assert_eq!(
        st.selection.batch.len(),
        1,
        "a single row selected (partial state)"
    );

    // Frame 5: click on the block checkbox → the partial state becomes
    // "all checked" (not "all unchecked").
    let mut raw = raw_1440();
    click(&mut raw, gencb);
    ctx.begin_pass(raw);
    show_editor(&ctx, &mut ws, &mut st);
    let _ = ctx.end_pass();
    assert_eq!(
        st.selection.batch.len(),
        expected.len(),
        "from the partial state, the block checkbox must check everything"
    );
}

/// The checkbox of a rank line selects/unselects the rows of the
/// rank (the other ranks of the block are not touched).
#[test]
fn rank_checkbox_selects_rows_of_rank() {
    let mut ws = test_workspace();
    let mut st = EditorState::default();
    st.view.category = ws
        .config()
        .category_id("MedStash")
        .expect("MedStash category");
    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);

    // StashMedicine_Smart: at least two rank tables (Newbie + Veteran).
    let mut by_block: std::collections::HashMap<BlockId, Vec<EntryId>> =
        std::collections::HashMap::new();
    for &id in &visible_indices(&ws, &st) {
        let e = ws.entry(id);
        if !e.is_delegation()
            && !e.fields.is_empty()
            && ws.block(e.block).name == "StashMedicine_Smart"
        {
            by_block.entry(e.block).or_default().push(id);
        }
    }
    let (&bkey, entries) = by_block
        .iter()
        .max_by_key(|(_, v)| v.len())
        .expect("visible StashMedicine_Smart blocks");
    // Ranks in the order of the file: the first two have their
    // rank lines on screen (the test window is 810 px tall;
    // a click off screen is ignored by egui).
    let ranks: Vec<String> = {
        let mut v: Vec<String> = Vec::new();
        for &id in entries {
            if let Some(r) = &ws.entry(id).ctx.rank_label {
                if !v.contains(r) {
                    v.push(r.clone());
                }
            }
        }
        v
    };
    assert!(
        ranks.len() >= 2,
        "StashMedicine_Smart doit avoir >= 2 rangs : {ranks:?}"
    );
    let (r0, r1) = (ranks[0].clone(), ranks[1].clone());
    let keys_of = |rank: &str| -> std::collections::HashSet<FieldId> {
        entries
            .iter()
            .filter(|&&id| ws.entry(id).ctx.rank_label.as_deref() == Some(rank))
            .filter_map(|&id| ws.entry(id).fields.first().copied())
            .collect()
    };
    let k0 = keys_of(&r0);
    let k1 = keys_of(&r1);
    assert!(
        !k0.is_empty() && !k1.is_empty(),
        "les deux rangs ont des rows"
    );
    st.view.expanded.insert(bkey);

    /// Position of the checkbox of a rank line: the label of the rank
    /// is the leftmost occurrence of the rank text (x = left+78 →
    /// checkbox left+22).
    fn rank_cb(out: &egui::FullOutput, rank: &str) -> Pos2 {
        let (x, y, h) = out
            .shapes
            .iter()
            .filter_map(|cs| {
                if let egui::Shape::Text(t) = &cs.shape {
                    if t.galley.text() == rank && t.pos.x > 300.0 && t.pos.y > 100.0 {
                        Some((t.pos.x, t.pos.y, t.galley.size().y))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
            .expect("rank label painted");
        pos2(x - 56.0, y + h / 2.0)
    }

    // Frame 1: positions of the rank lines.
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();
    let cb0 = rank_cb(&out, &r0);

    // Frame 2: click on the checkbox of rank r0 → only its rows.
    let mut raw = raw_1440();
    click(&mut raw, cb0);
    ctx.begin_pass(raw);
    show_editor(&ctx, &mut ws, &mut st);
    let _ = ctx.end_pass();
    assert_eq!(
        st.selection.batch, k0,
        "the rank checkbox must only select its rows"
    );

    // Frame 3: click on the checkbox of rank r1 → both ranks checked.
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();
    let cb1 = rank_cb(&out, &r1);
    let mut raw = raw_1440();
    click(&mut raw, cb1);
    ctx.begin_pass(raw);
    show_editor(&ctx, &mut ws, &mut st);
    let _ = ctx.end_pass();
    let mut both = k0;
    both.extend(k1.iter());
    assert_eq!(st.selection.batch, both, "both ranks must be selected");

    // Frame 4: uncheck r0 → only r1 remains.
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();
    let cb0 = rank_cb(&out, &r0);
    let mut raw = raw_1440();
    click(&mut raw, cb0);
    ctx.begin_pass(raw);
    show_editor(&ctx, &mut ws, &mut st);
    let _ = ctx.end_pass();
    assert_eq!(
        st.selection.batch, k1,
        "unchecking r0 must not touch the rows of r1"
    );
}
