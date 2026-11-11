//! behaviour — reset, min/max validation, collapsed blocks, spawn chances — headless UI tests (egui without GPU).

use crate::domain::value::{self, parse_percent, percent_display, percent_to_cfg};
use crate::domain::{BlockId, EntryId, FieldId, FieldKind};
use crate::ui::*;
use egui::pos2;

use super::common::*;
// ---- reset: global button + per-row button --------------------------
#[test]
fn bottom_reset_without_selection_resets_all() {
    let mut ws = test_workspace();
    let st = EditorState::default();
    assert!(st.selection.batch.is_empty(), "no row selected");
    let keys: Vec<FieldId> = ws.field_ids().take(2).collect();
    for k in keys {
        let v = if ws.field(k).original() == "0" {
            "1"
        } else {
            "0"
        };
        ws.set_text(k, v);
    }
    assert_eq!(ws.dirty_count(), 2);
    if st.selection.batch.is_empty() {
        ws.reset_all();
    } else {
        crate::ui::ops::reset_entries_from_keys(&mut ws, &st.selection.batch);
    }
    assert_eq!(
        ws.dirty_count(),
        0,
        "global reset expected when no row is selected"
    );
}

#[test]
fn row_reset_button_resets_dirty_row() {
    let mut ws = test_workspace();
    let mut st = EditorState::default();
    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);

    // Visible entry, non-delegation, with fields; take the first
    // occurrence of a name so we can locate its row among the painted
    // shapes (topmost row carrying this name).
    let entries = visible_entries(&ws, &st);
    let first = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| !e.is_delegation() && !e.fields.is_empty())
        .find(|(i, e)| entries[..*i].iter().all(|o| o.sid != e.sid))
        .map(|(_, e)| e.clone())
        .expect("visible entry with fields");
    let key = first.fields[0];
    // Blocks are collapsed by default: expand the one of `first`.
    st.view.expanded.insert(first.block);

    // Y positions of the row bottom lines (long, horizontal):
    // used to check that the reset button does not shift the layout.
    fn row_bottoms(out: &egui::FullOutput) -> Vec<f32> {
        let mut ys: Vec<f32> = out
            .shapes
            .iter()
            .filter_map(|cs| {
                if let egui::Shape::LineSegment { points, .. } = &cs.shape {
                    if (points[1].x - points[0].x) > 1000.0
                        && (points[1].y - points[0].y).abs() < 1.0
                        && points[0].y > 100.0
                        && points[0].y < 700.0
                    {
                        Some(points[0].y)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
        ys
    }

    // Frame 1 (clean): layout reference + row location.
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();
    let bottoms_clean = row_bottoms(&out);
    // `t.pos` is the top-left corner of the galley: the row center
    // (cy) is at pos.y + height/2.
    let (name_pos, galley_h) = out
        .shapes
        .iter()
        .filter_map(|cs| {
            if let egui::Shape::Text(t) = &cs.shape {
                if t.galley.text() == first.sid && t.pos.x > 100.0 && t.pos.y > 100.0 {
                    Some((t.pos, t.galley.size().y))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .min_by(|a, b| a.0.y.partial_cmp(&b.0.y).unwrap())
        .expect("row name not painted");
    let cy = name_pos.y + galley_h / 2.0;
    // Right edge of the row: the row bottom line (painted at cy+18)
    // spans the full width.
    let row_right = out
        .shapes
        .iter()
        .filter_map(|cs| {
            if let egui::Shape::LineSegment { points, .. } = &cs.shape {
                if (points[1].y - (cy + 18.0)).abs() < 1.0 && points[1].x > 1000.0 {
                    Some(points[1].x)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .next()
        .expect("row right edge not painted");

    // Modify a field: the reset button appears on the next frame.
    let v = if ws.field(key).original() == "0" {
        "1"
    } else {
        "0"
    };
    ws.set_text(key, v);
    assert!(ws.field(key).is_dirty());

    // Frame 2 (button visible): the layout must be identical to frame 1.
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();
    assert_eq!(
        row_bottoms(&out),
        bottoms_clean,
        "the presence of the reset button must not move the rows"
    );
    let btn = pos2(row_right - 26.0, cy);

    // Frame 3: click on the row reset button.
    let mut raw = raw_1440();
    raw.events.push(egui::Event::PointerButton {
        pos: btn,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    });
    raw.events.push(egui::Event::PointerButton {
        pos: btn,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    });
    ctx.begin_pass(raw);
    show_editor(&ctx, &mut ws, &mut st);
    let _ = ctx.end_pass();

    assert!(
        !ws.field(key).is_dirty(),
        "the row reset button must restore the field to its original value"
    );
    assert!(
        st.selection.batch.is_empty(),
        "clicking the row reset must not select the row"
    );
}

/// Blocks are collapsed by default (many items): their rows are only
/// painted once the triangle has expanded the block; clicking the
/// triangle acts on ALL rows of the block.
#[test]
fn collapsed_block_hides_all_rows() {
    let mut ws = test_workspace();
    let mut st = EditorState::default();
    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);

    // Visible block with the most entries (>= 2, non-delegations).
    let visible = visible_indices(&ws, &st);
    let mut by_block: std::collections::HashMap<BlockId, Vec<EntryId>> =
        std::collections::HashMap::new();
    for &id in &visible {
        let e = ws.entry(id);
        by_block.entry(e.block).or_default().push(id);
    }
    let (bkey, block_entries) = by_block
        .iter()
        .filter(|(_, v)| v.len() >= 2 && v.iter().all(|&id| !ws.entry(id).is_delegation()))
        .max_by_key(|(_, v)| v.len())
        .expect("a block with >= 2 non-delegation entries");
    let names: Vec<String> = block_entries
        .iter()
        .map(|&id| ws.entry(id).sid.clone())
        .collect();

    /// Texts painted in the table area (x > 260: right of the left
    /// panel; y > 90: below the top bar).
    fn table_texts(out: &egui::FullOutput) -> Vec<String> {
        out.shapes
            .iter()
            .filter_map(|cs| {
                if let egui::Shape::Text(t) = &cs.shape {
                    if t.pos.x > 260.0 && t.pos.y > 90.0 {
                        Some(t.galley.text().to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    // Frame 1: collapsed by default — no entry of the block is painted,
    // the generator row remains.
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();
    let texts1 = table_texts(&out);
    for n in &names {
        assert!(
            !texts1.iter().any(|t| t == n),
            "entry {n:?} painted while the block is collapsed by default"
        );
    }
    // Generator row: its name is painted at rect.left()+60 (LEFT_CENTER,
    // x ≈ 312); item names are at rect.left()+78 (x ≈ 330).
    let block_name = ws.block(*bkey).name.clone();
    let (gen_pos, gen_h) = out
        .shapes
        .iter()
        .filter_map(|cs| {
            if let egui::Shape::Text(t) = &cs.shape {
                if t.galley.text() == block_name && t.pos.x > 260.0 && t.pos.x < 800.0 {
                    Some((t.pos, t.galley.size().y))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .min_by(|a, b| a.0.y.partial_cmp(&b.0.y).unwrap())
        .expect("generator name not painted");
    let tri = pos2(gen_pos.x - 60.0 + 42.0, gen_pos.y + gen_h / 2.0);

    /// Y (top of the galley) of the generator rows below `y0`. The
    /// generator name is at x ≈ 312 (rect.left()+60, after the checkbox)
    /// and the item name at x ≈ 330 (rect.left()+78); the ⚠ icon is at
    /// x = 306 — so the generator name is isolated with (309, 320).
    fn gens_below(out: &egui::FullOutput, y0: f32) -> Vec<f32> {
        out.shapes
            .iter()
            .filter_map(|cs| {
                if let egui::Shape::Text(t) = &cs.shape {
                    if t.pos.x > 309.0 && t.pos.x < 320.0 && t.pos.y > y0 {
                        Some(t.pos.y)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    /// Item row names between `from` and `to`: name column at
    /// `SIDEBAR + ITEM_NAME_X` (x ≈ 358). One text per row: the leftmost
    /// one (the name, before the optional rank label).
    /// Texts of the same row are clustered by y proximity (≤ 8px: the
    /// 13px name and the 10px rank label have slightly different
    /// `pos.y`); two adjacent rows are spaced 36px apart.
    fn rows_between(out: &egui::FullOutput, from: f32, to: f32) -> Vec<String> {
        let mut texts: Vec<(f32, f32, String)> = out
            .shapes
            .iter()
            .filter_map(|cs| {
                if let egui::Shape::Text(t) = &cs.shape {
                    if t.pos.x > 340.0 && t.pos.x < 590.0 && t.pos.y > from && t.pos.y < to {
                        Some((t.pos.y, t.pos.x, t.galley.text().to_string()))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        texts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        // (cluster y, min x, text at min x)
        let mut rows: Vec<(f32, f32, String)> = Vec::new();
        for (y, x, s) in texts {
            match rows.last_mut() {
                Some(last) if y - last.0 <= 8.0 => {
                    if x < last.1 {
                        last.1 = x;
                        last.2 = s;
                    }
                }
                _ => rows.push((y, x, s)),
            }
        }
        rows.into_iter().map(|(_, _, s)| s).collect()
    }

    // Frame 2: click on the triangle.
    let mut raw = raw_1440();
    raw.events.push(egui::Event::PointerButton {
        pos: tri,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    });
    raw.events.push(egui::Event::PointerButton {
        pos: tri,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    });
    ctx.begin_pass(raw);
    show_editor(&ctx, &mut ws, &mut st);
    let _ = ctx.end_pass();
    assert!(
        st.view.expanded.contains(bkey),
        "clicking the triangle must expand the block"
    );

    // Frame 3: all entries of the block are painted, the generator row
    // remains.
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();
    let (gen3, h3) = out
        .shapes
        .iter()
        .filter_map(|cs| {
            if let egui::Shape::Text(t) = &cs.shape {
                if t.galley.text() == block_name && t.pos.x > 260.0 && t.pos.x < 800.0 {
                    Some((t.pos, t.galley.size().y))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .min_by(|a, b| a.0.y.partial_cmp(&b.0.y).unwrap())
        .expect("the generator row must stay visible");
    let bottom3 = gen3.y + h3;
    let next3 = gens_below(&out, bottom3)
        .into_iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(f32::INFINITY);
    let rows_3 = rows_between(&out, bottom3, next3);
    assert_eq!(
        rows_3.len(),
        names.len(),
        "expanded block: its rows must be shown (visible: {rows_3:?})"
    );
}

/// Committing a MinCount > MaxCount (or a MaxCount < MinCount) must be
/// rejected at commit time, with a message — not left as a validation error.
#[test]
fn min_max_pair_rejects_incoherent_commit() {
    let mut ws = test_workspace();
    let mut st = EditorState::default();

    // Visible entry with a Min/Max pair.
    let entries = visible_entries(&ws, &st);
    let e = entries
        .iter()
        .find(|e| !e.is_delegation() && ws.count_pair(e).is_some())
        .cloned()
        .expect("visible entry with Min/Max pair");
    let (min_k, max_k) = ws.count_pair(&e).unwrap();
    let max_v = value::parse_value(ws.field(max_k).value()).expect("Max readable");
    let min_v = value::parse_value(ws.field(min_k).value()).expect("Min readable");

    // Min > Max: rejected.
    let bad = format!("{}", (max_v + 10.0) as i64);
    do_commit(&mut ws, &mut st, min_k, &bad);
    let msg = format!(
        "a Min ({bad}) > Max ({}) must be rejected at commit",
        max_v as i64
    );
    assert!(!ws.field(min_k).is_dirty(), "{msg}");
    assert!(st.msg.is_some(), "an error message must be shown");
    assert_eq!(ws.validation_errors().len(), 0);

    // Symmetric: Max < Min: rejected (if Min > 0).
    st.msg = None;
    if min_v > 0.0 {
        let bad = format!("{}", (min_v - 1.0) as i64);
        do_commit(&mut ws, &mut st, max_k, &bad);
        let msg = format!(
            "a Max ({bad}) < Min ({}) must be rejected at commit",
            min_v as i64
        );
        assert!(!ws.field(max_k).is_dirty(), "{msg}");
        assert!(st.msg.is_some(), "an error message must be shown");
    }

    // Coherent value (Min = Max): accepted, no message.
    st.msg = None;
    let ok = format!("{}", max_v as i64);
    do_commit(&mut ws, &mut st, min_k, &ok);
    assert!(st.msg.is_none(), "a Min ≤ Max must be accepted");
    assert_eq!(ws.validation_errors().len(), 0);
}

// ------------------------------------------------------------------
// Spawn chances: the UI works in percentage, the cfg in [0, 1]f
// ------------------------------------------------------------------

#[test]
fn spawn_chance_percent_roundtrip() {
    // Display: cfg text → percentage.
    assert_eq!(percent_display("0.1f"), "10%");
    assert_eq!(percent_display("1.f"), "100%");
    assert_eq!(percent_display("0.f"), "0%");
    assert_eq!(percent_display("0.01f"), "1%");
    assert_eq!(percent_display("0.5"), "50%");
    // Input: percentage → cfg text (vanilla format, "f" suffix).
    assert_eq!(percent_to_cfg(parse_percent("25").unwrap()), "0.25f");
    assert_eq!(percent_to_cfg(parse_percent("25%").unwrap()), "0.25f");
    assert_eq!(percent_to_cfg(parse_percent("100").unwrap()), "1.f");
    assert_eq!(percent_to_cfg(parse_percent("0").unwrap()), "0.f");
    assert_eq!(percent_to_cfg(parse_percent("12.5").unwrap()), "0.125f");
    assert!(parse_percent("abc").is_none()); // non-numeric: rejected

    // f64 regressions: the "x * 100" and "y / 100" multiplications/
    // divisions produced artifacts (28.999999999999996%, 0.33299999999999996f).
    assert_eq!(percent_display("0.29f"), "29%", "0.29f must display 29%");
    assert_eq!(percent_display("0.57f"), "57%");
    assert_eq!(percent_display("0.58f"), "58%");
    assert_eq!(percent_display("0.125f"), "12.5%");
    assert_eq!(percent_to_cfg(parse_percent("33.3").unwrap()), "0.333f");
    assert_eq!(percent_to_cfg(parse_percent("7.5").unwrap()), "0.075f");

    // Round-trip display → cfg → display, every 0.1%.
    for i in 0..=1000 {
        let pct_in = format!("{:0.1}", i as f64 / 10.0); // "0.0" .. "100.0"
        let cfg = percent_to_cfg(parse_percent(&pct_in).unwrap());
        let Some(v) = value::parse_value(&cfg) else {
            panic!("{pct_in}% → {cfg:?} unreadable")
        };
        assert!(
            (v - i as f64 / 1000.0).abs() < 5e-7,
            "{pct_in}% → {cfg:?} = {v}"
        );
        let disp = percent_display(&cfg);
        assert_eq!(
            percent_to_cfg(parse_percent(&disp).unwrap()),
            cfg,
            "{pct_in}% : {disp:?} → instable ({cfg:?})"
        );
        // Canonical display: integer without decimal, 1 decimal otherwise.
        assert_eq!(
            disp,
            format!("{}%", i as f64 / 10.0),
            "{pct_in}% → {disp:?}"
        );
    }

    // Commit: "25" in a MinSpawnChance stores "0.25f" (export ok).
    let mut ws = test_workspace();
    let mut st = EditorState::default();
    // Spawn chances are group fields (scope "Set N"), not attached to
    // entries: search in the field list.
    let key = ws
        .field_ids()
        .find(|&k| ws.field(k).kind == FieldKind::MinSpawnChance)
        .expect("a MinSpawnChance field");
    assert_ne!(ws.field(key).original(), "0.25f");
    do_commit(&mut ws, &mut st, key, "25");
    assert_eq!(
        ws.field(key).current(),
        Some("0.25f"),
        "the export must carry the cfg format"
    );
    assert!(ws.field(key).is_valid());
    assert!(st.msg.is_none());

    // Out of range: rejected with a percentage message, value kept.
    do_commit(&mut ws, &mut st, key, "150");
    assert!(st.msg.is_some());
    assert_eq!(ws.field(key).current(), Some("0.25f"));

    // Identical re-entry: idempotent, no error.
    st.msg = None;
    do_commit(&mut ws, &mut st, key, "25%");
    assert_eq!(ws.field(key).current(), Some("0.25f"));
    assert!(st.msg.is_none());
}

// ---- search: "×" clear button ---------------------------
/// The two diagonal segments of the "×" cross, painted by the Painter
/// in the button area (right of the search field, filter bar — the
/// field is right-aligned on a 1440-wide screen: button center ≈ (1416, 74)).
/// Search tightened to this area: no sweep of all shapes.
fn clear_cross_lines(out: &egui::FullOutput) -> (egui::Pos2, egui::Pos2) {
    let mut pts = Vec::new();
    for cs in &out.shapes {
        if let egui::Shape::LineSegment { points, .. } = &cs.shape {
            let c = points[0] + (points[1] - points[0]) * 0.5;
            if (1408.0..1424.0).contains(&c.x) && (66.0..82.0).contains(&c.y) {
                pts.push(c);
            }
        }
    }
    assert_eq!(pts.len(), 2, "the \"×\" cross = two diagonal segments");
    (pts[0], pts[1])
}

#[test]
fn search_clear_button_erases_the_query() {
    let mut ws = test_workspace();
    let mut st = EditorState::default();
    st.view.search = "medkit".into();
    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);
    let raw = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1440.0, 810.0))),
        ..Default::default()
    };
    ctx.begin_pass(raw);
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();
    // The cross is painted: two diagonal segments whose intersection
    // is the geometric center of the button, at the field center
    // (≈ (1416, 74)).
    let (a, b) = clear_cross_lines(&out);
    let center = a + (b - a) * 0.5;
    assert!(
        (center - pos2(1416.0, 74.0)).length() < 1.0,
        "cross intersection at {center:?}, expected ≈ (1416, 74)"
    );

    // Frame 2: click at the button center → the field is cleared.
    let mut raw = raw_1440();
    click(&mut raw, center);
    ctx.begin_pass(raw);
    show_editor(&ctx, &mut ws, &mut st);
    let _ = ctx.end_pass();
    assert!(
        st.view.search.is_empty(),
        "clicking on \"×\" must clear the field"
    );
}

#[test]
fn search_clear_button_is_painted_when_empty() {
    // Empty field: the cross stays painted (greyed out via `add_enabled`),
    // and clicking it does nothing (disabled button → no clearing).
    let mut ws = test_workspace();
    let mut st = EditorState::default();
    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);
    let raw = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1440.0, 810.0))),
        ..Default::default()
    };
    ctx.begin_pass(raw);
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();
    let (a, b) = clear_cross_lines(&out);
    let center = a + (b - a) * 0.5;

    // Click on the disabled button: no effect.
    let mut raw = raw_1440();
    click(&mut raw, center);
    ctx.begin_pass(raw);
    show_editor(&ctx, &mut ws, &mut st);
    let _ = ctx.end_pass();
    assert!(st.view.search.is_empty());
}
