//! render — painting / layout — headless UI tests (egui without GPU).

use crate::ui::*;
use egui::{pos2, vec2, Color32, Rect};

use super::common::*;
#[test]
fn topbar_is_painted() {
    let (texts, info) = painted_texts();
    let dump: String = texts
        .iter()
        .map(|(s, x, y)| format!("{x:4},{y:4}  {s}"))
        .collect::<Vec<_>>()
        .join("\n");
    let ok = texts.iter().any(|(s, _, y)| s == "zoneEDIT" && *y < 52);
    assert!(ok, "topbar 'zoneEDIT' missing or misplaced. {info}\n{dump}");
}

#[test]
fn bottom_bars_do_not_overlap() {
    // The batch bar must be above the statusbar (32 px at the bottom)
    // and must not overflow under it (regression: the vertical item_spacing
    // of the central panel was pushing the batch under the statusbar).
    let (out, _) = painted_frame();
    let bottom_limit = 810.0_f32 - 32.0;
    let offenders: Vec<Rect> = out
        .shapes
        .iter()
        .filter_map(|cs| match &cs.shape {
            egui::Shape::Rect(r) => Some((r.fill, r.rect)),
            _ => None,
        })
        .filter(|(fill, r)| {
            (*fill == c::BG || *fill == c::PANEL)
                && r.width() > 1000.0
                && r.top() < bottom_limit
                && r.bottom() > bottom_limit + 0.5
        })
        .map(|(_, r)| r)
        .collect();
    assert!(
        offenders.is_empty(),
        "large rects spilling under the status bar (bottom > {bottom_limit}): {offenders:?}"
    );
    let (texts, info) = painted_texts();
    let dump: String = texts
        .iter()
        .map(|(s, x, y)| format!("{x:4},{y:4}  {s}"))
        .collect::<Vec<_>>()
        .join("\n");
    let batch_y = texts
        .iter()
        .find(|(s, _, _)| s.contains("row(s) selected"))
        .map(|(_, _, y)| *y)
        .expect("batch bar label missing");
    let status_y = texts
        .iter()
        .find(|(s, _, _)| s.contains("pending change(s)"))
        .map(|(_, _, y)| *y)
        .expect("status bar label missing");
    let screen_h = 810i32;
    assert!(
        batch_y < screen_h - 32 && status_y > screen_h - 32 && batch_y < status_y,
        "batch/status overlap: batch_y={batch_y} status_y={status_y}. {info}\n{dump}"
    );
}

#[test]
fn table_rows_are_painted() {
    let (texts, info) = painted_texts();
    let dump: String = texts
        .iter()
        .map(|(s, x, y)| format!("{x:4},{y:4}  {s}"))
        .collect::<Vec<_>>()
        .join("\n");
    let ws = test_workspace();
    let med = ws
        .config()
        .category_id("MedCrate")
        .expect("MedCrate category");
    let bid = ws
        .entry_ids()
        .find(|&id| ws.in_category(id, med))
        .expect("med crate entry");
    let block = ws.block(ws.entry(bid).block).name.clone();
    let ok = texts
        .iter()
        .any(|(s, x, y)| s == &block && *x > 250 && *y > 100 && *y < 760);
    assert!(ok, "first generator row missing from body. {info}\n{dump}");
}

#[test]
fn cell_edit_typing_repro() {
    let mut ws = test_workspace();
    let mut st = EditorState::default();
    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);

    let entries = visible_entries(&ws, &st);
    let first = entries
        .iter()
        .find(|e| !e.fields.is_empty())
        .expect("entry with fields");
    let key = first.fields[0];
    let val = ws.field(key).value().to_string();
    // The blocks are collapsed by default: we expand the one of `first`.
    st.view.expanded.insert(first.block);

    // Frame 1: locate the center of the painted cell.
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();
    let cell = out
        .shapes
        .iter()
        .find_map(|cs| {
            if let egui::Shape::Text(t) = &cs.shape {
                if t.galley.text() == val && t.pos.x > 260.0 && t.pos.y > 100.0 {
                    Some(t.pos + t.galley.size() / 2.0)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .expect("cell value not painted");
    eprintln!("[repro] click cell at {cell:?} value={val:?} key={key:?}");

    // Frame 2: click on the cell.
    let mut raw = raw_1440();
    raw.events.push(egui::Event::PointerButton {
        pos: cell,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    });
    raw.events.push(egui::Event::PointerButton {
        pos: cell,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    });
    ctx.begin_pass(raw);
    show_editor(&ctx, &mut ws, &mut st);
    let _ = ctx.end_pass();
    eprintln!(
        "[repro] after click: editing={:?} focus={:?}",
        st.editing.cell,
        ctx.memory(|m| m.focused())
    );
    assert_eq!(st.editing.cell, Some(key), "editor did not open on click");

    // Frame 3: type "5".
    let mut raw = raw_1440();
    raw.events.push(egui::Event::Text("5".to_string()));
    ctx.begin_pass(raw);
    show_editor(&ctx, &mut ws, &mut st);
    let _ = ctx.end_pass();
    let cur = ws.field(key).current().map(str::to_string);
    eprintln!(
        "[repro] after typing '5': editing={:?} focus={:?} committed={cur:?}",
        st.editing.cell,
        ctx.memory(|m| m.focused())
    );
    assert_eq!(
        st.editing.cell,
        Some(key),
        "editor closed right after typing"
    );

    // Frame 4: type "3" (the buffer must persist between frames).
    let mut raw = raw_1440();
    raw.events.push(egui::Event::Text("3".to_string()));
    ctx.begin_pass(raw);
    show_editor(&ctx, &mut ws, &mut st);
    let _ = ctx.end_pass();
    assert_eq!(
        st.editing.cell,
        Some(key),
        "editor closed right after second char"
    );

    // Frame 5: Enter to validate.
    let mut raw = raw_1440();
    raw.events.push(egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        modifiers: Default::default(),
        repeat: false,
    });
    ctx.begin_pass(raw);
    show_editor(&ctx, &mut ws, &mut st);
    let _ = ctx.end_pass();
    let cur = ws.field(key).current().map(str::to_string);
    eprintln!(
        "[repro] after Enter: editing={:?} focus={:?} committed={cur:?}",
        st.editing.cell,
        ctx.memory(|m| m.focused())
    );
    assert_eq!(st.editing.cell, None, "editor did not close on Enter");
    assert_eq!(
        cur.as_deref(),
        Some("53"),
        "value not committed to 53 (full selection + persistent buffer expected)"
    );
}

// ------------------------------------------------------------------
// The rank filter selects the displayed/edited table
// ------------------------------------------------------------------

#[test]
fn rank_filter_selects_rank_table() {
    let mut ws = test_workspace();
    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);
    let mut st = EditorState::default();
    st.view.category = ws
        .config()
        .category_id("MedStash")
        .expect("MedStash category");
    // The blocks are collapsed by default: we expand all the ones of the view.
    for e in visible_entries(&ws, &st) {
        st.view.expanded.insert(e.block);
    }

    // Texts painted in the table zone (excluding the top bar).
    let table_texts = |out: &egui::FullOutput| -> Vec<String> {
        out.shapes
            .iter()
            .filter_map(|cs| {
                if let egui::Shape::Text(t) = &cs.shape {
                    if t.pos.y > 100.0 {
                        Some(t.galley.text().to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    };

    // Real set labels: "Set <n> · <item>" — any other "Set …" label
    // (suffix not starting with a digit, e.g. a batch button enabled
    // by a config override) does not count.
    let set_labels = |ts: &[String]| -> Vec<String> {
        ts.iter()
            .filter(|t| {
                let Some(rest) = t.strip_prefix("Set ") else {
                    return false;
                };
                rest.chars().next().is_some_and(|c| c.is_ascii_digit())
            })
            .cloned()
            .collect()
    };

    // --- All mode: the "Set N" group lines are drawn —
    // they carry the editable spawn chances (Min/Max Spawn);
    // the rank is also a small label after the item name.
    st.view.difficulty = 0;
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();
    let all = table_texts(&out);
    assert!(
        !set_labels(&all).is_empty(),
        "the 'Set N' group lines must be drawn (they carry Min/Max Spawn)"
    );
    assert!(
        all.iter().any(|t| t == "Newbie"),
        "in All mode, the rank label must be visible after the name"
    );

    // --- Veteran mode: only the Veteran tables, no Newbie text.
    st.view.difficulty = ws
        .config()
        .ranks
        .iter()
        .position(|r| r == "Veteran")
        .map(|i| 1u32 << i)
        .expect("Veteran rank in the config");
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();
    let vet = table_texts(&out);
    let bad: Vec<String> = vet
        .iter()
        .filter(|t| t.contains("Newbie"))
        .cloned()
        .collect();
    assert!(
        bad.is_empty(),
        "in Veteran mode, no Newbie text must remain: {bad:?}"
    );
    // StashMedicine_Smart has a Veteran table: displayed, with its items.
    assert!(
        vet.iter().any(|t| t == "StashMedicine_Smart"),
        "the StashMedicine_Smart block (Veteran table) must be displayed"
    );
    assert!(
        vet.iter().any(|t| t == "Medkit"),
        "the items of the Veteran table must be displayed"
    );
    // StashMedicine_Cheap only has a Newbie table: fully hidden.
    assert!(
        !vet.iter().any(|t| t.contains("StashMedicine_Cheap")),
        "StashMedicine_Cheap (Newbie only) must be hidden in Veteran mode"
    );
    // The Veteran table of StashMedicine_Smart displays its set lines.
    assert!(
        !set_labels(&vet).is_empty(),
        "the set lines of the Veteran table must be drawn"
    );
}

// ------------------------------------------------------------------
// Difficulty filter: in rank mode, the entries WITHOUT rank (neither a
// Rank field, nor a rank in the block name) are hidden; the "No Rank" mode
// displays ONLY the entries without rank.
// ------------------------------------------------------------------

#[test]
fn rank_filter_hides_rankless_and_no_rank_mode() {
    let mut ws = test_workspace();
    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);
    let mut st = EditorState::default();

    let table_texts = |out: &egui::FullOutput| -> Vec<String> {
        out.shapes
            .iter()
            .filter_map(|cs| {
                if let egui::Shape::Text(t) = &cs.shape {
                    if t.pos.y > 100.0 {
                        Some(t.galley.text().to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    };
    let ranks = ws.config().ranks.clone();
    // Rank bit in the `st.view.difficulty` mask.
    let rank_d = |name: &str| -> u32 {
        ranks
            .iter()
            .position(|r| r == name)
            .map(|i| 1u32 << i)
            .expect("rang dans la config")
    };

    // --- MedCrate: the 3 blocks of the fixture have a rank in their name
    //     (Newbie / Experienced / Experienced-delegation).
    st.view.category = ws.config().category_id("MedCrate").expect("MedCrate");
    for e in visible_entries(&ws, &st) {
        st.view.expanded.insert(e.block);
    }

    // Newbie mode: only the Newbie block remains, no trace of Experienced.
    st.view.difficulty = rank_d("Newbie");
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let nb = table_texts(&ctx.end_pass());
    assert!(
        nb.iter().any(|t| t == "DestructibleStash_Medkit_Newbie"),
        "le bloc Newbie doit rester en mode Newbie : {nb:?}"
    );
    assert!(
        !nb.iter().any(|t| t.contains("Experienced")),
        "aucun bloc Experienced ne doit rester en mode Newbie : {nb:?}"
    );

    // No Rank mode (only active filter): all the MedCrate blocks have a
    // rank → empty table.
    st.view.difficulty = 0;
    st.view.no_rank = true;
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let nr = table_texts(&ctx.end_pass());
    assert!(
        !nr.iter().any(|t| t.contains("DestructibleStash")),
        "in No Rank mode, the blocks with a rank in the name must be hidden: {nr:?}"
    );

    // --- MedDirect: PlayerItemGenerator has NO rank in its name.
    st.view.category = ws.config().category_id("MedDirect").expect("MedDirect");
    st.view.no_rank = false;
    st.view.difficulty = 0;
    for e in visible_entries(&ws, &st) {
        st.view.expanded.insert(e.block);
    }
    st.view.difficulty = rank_d("Experienced");
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let e2 = table_texts(&ctx.end_pass());
    assert!(
        !e2.iter().any(|t| t.contains("PlayerItemGenerator")),
        "without a rank in the name, the block must be hidden in rank mode: {e2:?}"
    );

    // No Rank mode: PlayerItemGenerator and its items reappear.
    st.view.difficulty = 0;
    st.view.no_rank = true;
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let nr2 = table_texts(&ctx.end_pass());
    assert!(
        nr2.iter().any(|t| t == "PlayerItemGenerator"),
        "in No Rank mode, PlayerItemGenerator must be displayed: {nr2:?}"
    );
    assert!(
        nr2.iter().any(|t| t == "Medkit"),
        "its items must be visible: {nr2:?}"
    );
}

// ------------------------------------------------------------------
// Rank sub-dropdown: click on the triangle = collapse/expand the
// rank section (sets + items); the rank is expanded by default.
// ------------------------------------------------------------------

#[test]
fn rank_row_is_collapsible_subdropdown() {
    let mut ws = test_workspace();
    let mut st = EditorState::default();
    st.view.category = ws
        .config()
        .category_id("MedStash")
        .expect("MedStash category");
    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);

    // StashMedicine_Cheap only has one table (Newbie): collapsing it hides
    // all its sets + items.
    let cheap_id = ws
        .entry_ids()
        .find(|&id| {
            let e = ws.entry(id);
            ws.block(e.block).name == "StashMedicine_Cheap" && e.itemset_label.is_some()
        })
        .expect("StashMedicine_Cheap entry with an itemset");
    let cheap = ws.entry(cheap_id).clone();
    let rank = cheap.ctx.rank_label.clone().expect("block rank (Newbie)");
    let rkey = (cheap.block, rank.clone());
    st.view.expanded.insert(cheap.block);

    /// Texts of the table zone (y > 100, under the topbar) with position.
    fn table_texts(out: &egui::FullOutput) -> Vec<(String, f32, f32)> {
        out.shapes
            .iter()
            .filter_map(|cs| {
                if let egui::Shape::Text(t) = &cs.shape {
                    if t.pos.y > 100.0 {
                        Some((t.galley.text().to_string(), t.pos.x, t.pos.y))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }
    /// "Set N · …" (not the "Set Chance…" / "Set Min/Max…" buttons).
    fn is_set_label(s: &str) -> bool {
        s.strip_prefix("Set ")
            .and_then(|rest| rest.chars().next())
            .is_some_and(|c| c.is_ascii_digit())
    }

    // Frame 1: rank expanded by default — rank label + sets visible.
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();
    let t1 = table_texts(&out);
    assert!(
        t1.iter().any(|(s, _, _)| is_set_label(s)),
        "the set lines must be visible (rank expanded by default)"
    );
    // The rank label is the leftmost occurrence (the rank badges
    // after the item names are further to the right).
    let (rank_x, rank_y) = t1
        .iter()
        .filter(|(s, _, _)| s == &rank)
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .map(|(_, x, y)| (*x, *y))
        .expect("rank label not painted");

    // Frame 2: click on the rank triangle (just to the left of the label).
    let tri = pos2(rank_x - 18.0, rank_y + 8.0);
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
        st.view.rank_collapsed.contains(&rkey),
        "le clic sur le triangle du rang doit replier la sous-section"
    );

    // Frame 3: rank collapsed — sets + items hidden, label still there.
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();
    let t2 = table_texts(&out);
    assert!(
        !t2.iter().any(|(s, _, _)| is_set_label(s)),
        "the sets of the collapsed rank must be hidden"
    );
    assert!(
        t2.iter()
            .any(|(s, x, _)| s == &rank && (*x - rank_x).abs() < 4.0),
        "the rank label must remain visible (collapsed triangle)"
    );

    // Frame 4: click again → the section comes back.
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
        !st.view.rank_collapsed.contains(&rkey),
        "the second click must re-expand the sub-section"
    );
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();
    assert!(
        table_texts(&out).iter().any(|(s, _, _)| is_set_label(s)),
        "the sets must reappear after re-expansion"
    );
}

// ------------------------------------------------------------------
// Option A: Min/Max SpawnChance columns of the medical stashes
// ------------------------------------------------------------------

#[test]
fn medstash_spawn_chance_columns() {
    let mut ws = test_workspace();
    let mut st = EditorState::default();
    st.view.category = ws
        .config()
        .category_id("MedStash")
        .expect("MedStash category");
    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);
    // The blocks are collapsed by default: we expand all the ones of the view.
    for e in visible_entries(&ws, &st) {
        st.view.expanded.insert(e.block);
    }

    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();
    let mut texts: Vec<(String, f32, f32)> = Vec::new();
    for cs in &out.shapes {
        if let egui::Shape::Text(t) = &cs.shape {
            texts.push((t.galley.text().to_string(), t.pos.x, t.pos.y));
        }
    }
    let dump: String = texts
        .iter()
        .map(|(s, x, y)| format!("{x:7.1},{y:7.1}  {s}"))
        .collect::<Vec<_>>()
        .join("\n");

    // Headers of the new columns (right side of the table). Short labels:
    // the header budget is the ~70 px column pitch ("Min %" / "Max %" sit
    // next to "Min Count" / "Max Count").
    assert!(
        texts.iter().any(|(s, x, _)| s == "MIN %" && *x > 1000.0),
        "MIN % header missing.\n{dump}"
    );
    assert!(
        texts.iter().any(|(s, x, _)| s == "MAX %" && *x > 1000.0),
        "MAX % header missing.\n{dump}"
    );

    // StashMedicine_Cheap: rank line + one line per set (2 sets
    // numbered "Set 3" in the vanilla data).
    assert!(
        texts
            .iter()
            .any(|(s, x, y)| s == "StashMedicine_Cheap" && *x > 250.0 && *y > 100.0),
        "StashMedicine_Cheap generator line missing.\n{dump}"
    );
    // Rank/set labels at x ≈ 330 (table edge + 78); the small
    // rank labels after the item names are at x ≥ 380.
    let n_rank = texts
        .iter()
        .filter(|(s, x, y)| s == "Newbie" && (300.0..370.0).contains(x) && *y > 100.0)
        .count();
    assert!(n_rank >= 1, "'Newbie' rank line missing.\n{dump}");
    // Positional labels: "Set 1 · Medkit" then "Set 2 · Bandage".
    assert!(
        texts
            .iter()
            .any(|(s, x, y)| s == "Set 1 · Medkit" && (300.0..370.0).contains(x) && *y > 100.0),
        "lignes de set absentes.\n{dump}"
    );
    assert!(
        texts
            .iter()
            .any(|(s, x, y)| s == "Set 2 · Bandage" && (300.0..370.0).contains(x) && *y > 100.0),
        "lignes de set absentes.\n{dump}"
    );

    // Spawn chance cells, displayed as percentages
    // (vanilla values "0.f" / "1.f" → "0%" / "100%"), in the
    // new columns (x > 1150), at the height of the sets.
    assert!(
        texts
            .iter()
            .any(|(s, x, y)| s == "0%" && *x > 1150.0 && *y > 100.0),
        "cellule MinSpawnChance absente.\n{dump}"
    );
    assert!(
        texts
            .iter()
            .any(|(s, x, y)| s == "100%" && *x > 1150.0 && *y > 100.0),
        "cellule MaxSpawnChance absente.\n{dump}"
    );
}

/// "ITEM TYPES" of the contextual sidebar to the "File" filter: with a
/// selected file, the categories absent from this file disappear
/// and the counters are limited to the file.
#[test]
fn spawn_source_sidebar_is_contextual_to_file_filter() {
    let mut ws = test_workspace();
    let mut st = EditorState::default();
    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);

    // A file whose per-category counters differ from the global ones.
    let f = (0..ws.files().len())
        .find(|&i| ws.category_counts(Some(i)) != ws.category_counts(None))
        .expect("a file with a different category profile");
    st.view.file_filter = Some(f);

    let mut r = raw_1440();
    r.screen_rect = Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1440.0, 810.0)));
    ctx.begin_pass(r);
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();

    // Texts painted in the sidebar (x < 250) and everywhere.
    let sidebar: Vec<String> = out
        .shapes
        .iter()
        .filter_map(|cs| {
            if let egui::Shape::Text(t) = &cs.shape {
                if t.pos.x < 250.0 {
                    Some(t.galley.text().to_string())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();
    let all: Vec<String> = out
        .shapes
        .iter()
        .filter_map(|cs| {
            if let egui::Shape::Text(t) = &cs.shape {
                Some(t.galley.text().to_string())
            } else {
                None
            }
        })
        .collect();

    let global = ws.category_counts(None);
    let per = ws.category_counts(Some(f));
    for (cat, n) in &per {
        let label = ws.config().category(*cat).short.clone();
        assert!(
            sidebar.iter().any(|t| t == &label),
            "category {cat:?} present in the file: label visible"
        );
        assert!(
            sidebar.iter().any(|t| t.as_str() == n.to_string()),
            "compteur {n:?} du fichier visible"
        );
    }
    for (cat, n) in &global {
        let in_file = per
            .iter()
            .find(|(c, _)| c == cat)
            .map(|(_, n)| *n)
            .unwrap_or(0);
        if *n > 0 && in_file == 0 {
            let label = ws.config().category(*cat).short.clone();
            assert!(
                !all.iter().any(|t| t == &label),
                "category {cat:?} absent from the file: label hidden"
            );
        }
    }
}

// ------------------------------------------------------------------
// Delegation highlighting: the delegation row, its source block and the
// target block share the same very light tint
// ------------------------------------------------------------------

#[test]
fn delegation_highlight_links_source_and_target() {
    let mut ws = test_workspace();
    let mut st = EditorState::default();
    let med = ws
        .config()
        .category_id("MedCrate")
        .expect("MedCrate category");
    st.view.category = med;
    // The blocks are collapsed by default: we expand all the ones of the view.
    for e in visible_entries(&ws, &st) {
        st.view.expanded.insert(e.block);
    }
    let d = compute_derived(&ws, &st);

    // Fixtures delegation:
    // DestructibleStash_Med_Experienced → DestructibleStash_Medkit_Experienced.
    let del = d
        .visible_idx
        .iter()
        .find(|&&id| {
            let e = ws.entry(id);
            e.is_delegation() && e.sid == "DestructibleStash_Medkit_Experienced"
        })
        .copied()
        .expect("delegation visible in MedCrate");
    let src = ws.entry(del).block;
    let tgt = ws
        .blocks()
        .iter()
        .position(|b| b.name == "DestructibleStash_Medkit_Experienced")
        .map(|i| crate::domain::BlockId(i as u32))
        .expect("target block present in the workspace");

    let tint = d
        .delegation_tint
        .get(&del)
        .copied()
        .expect("tint of the delegation row");
    assert_ne!(tint, Color32::TRANSPARENT, "teinte transparente");
    assert_eq!(
        d.block_tint.get(&src),
        Some(&tint),
        "source block (DestructibleStash_Med_Experienced) not tinted"
    );
    assert_eq!(
        d.block_tint.get(&tgt),
        Some(&tint),
        "target block (DestructibleStash_Medkit_Experienced) not tinted"
    );

    // End-to-end: the tint is actually painted on the frame.
    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();
    let painted = out
        .shapes
        .iter()
        .any(|cs| matches!(&cs.shape, egui::Shape::Rect(r) if r.fill == tint));
    assert!(painted, "delegation tint never painted on the frame");
}

// ------------------------------------------------------------------
// "Show refs": the "N refs" badge is hidden by default and
// displayed when the checkbox is checked (filter bar)
// ------------------------------------------------------------------

fn frame_texts(ws: &mut crate::domain::Workspace, st: &mut EditorState) -> Vec<String> {
    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, ws, st);
    ctx.end_pass()
        .shapes
        .iter()
        .filter_map(|cs| {
            if let egui::Shape::Text(t) = &cs.shape {
                Some(t.galley.text().to_string())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn refs_bubbles_follow_show_refs_flag() {
    let mut ws = test_workspace();
    let mut st = EditorState::default();
    let med = ws
        .config()
        .category_id("MedCrate")
        .expect("MedCrate category");
    st.view.category = med;
    // The fixtures delegation gives "1 refs" to the source block.
    for e in visible_entries(&ws, &st) {
        st.view.expanded.insert(e.block);
    }

    // Default: the badge is absent (and the "Show refs" label is there).
    assert!(!st.view.show_refs, "expected default: show_refs = false");
    let off = frame_texts(&mut ws, &mut st);
    assert!(
        !off.iter().any(|t| t == "1 refs"),
        "'1 refs' badge visible while Show refs is disabled"
    );
    assert!(
        off.iter().any(|t| t == "Show refs"),
        "'Show refs' label absent from the filter bar"
    );

    // Checked: the badge appears.
    st.view.show_refs = true;
    let on = frame_texts(&mut ws, &mut st);
    assert!(
        on.iter().any(|t| t == "1 refs"),
        "'1 refs' badge absent while Show refs is enabled"
    );
}

#[test]
fn anomalies_render_hierarchy() {
    let mut ws = test_workspace();
    let section_idx = ws
        .config()
        .sections()
        .iter()
        .position(|s| s == "STATS")
        .expect("STATS section present");
    let anoms = ws
        .config()
        .category_id("Anomalies")
        .expect("Anomalies category present");
    let mut st = EditorState::default();
    st.view.section = section_idx;
    st.view.category = anoms;

    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();

    let texts: Vec<(String, i32, i32)> = out
        .shapes
        .iter()
        .filter_map(|cs| match &cs.shape {
            egui::Shape::Text(t) => Some((
                t.galley.text().to_string(),
                t.pos.x.round() as i32,
                t.pos.y.round() as i32,
            )),
            _ => None,
        })
        .collect();
    let dump = texts
        .iter()
        .map(|(s, x, y)| format!("{x:4},{y:4}  {s}"))
        .collect::<Vec<_>>()
        .join("\n");

    // Root anomaly name (ElectroAnomaly) painted once in the body, at the
    // standard item name position (no redundant generator header).
    let root_hits = texts
        .iter()
        .filter(|(t, x, y)| t == "ElectroAnomaly" && *x > 100 && *y > 100 && *y < 760)
        .count();
    assert_eq!(
        root_hits, 1,
        "ElectroAnomaly root must appear once, got {root_hits}.\n{dump}"
    );

    // Its child (ElectroAnomaly_Dynamic) is rendered BELOW the parent, with a
    // "↳" marker glued to its name (exactly like a delegation row) and the
    // name indented 1×HIER_INDENT to the right — no icon, no separate arrow.
    let (root_x, root_y) = texts
        .iter()
        .find(|(t, x, _)| t == "ElectroAnomaly" && *x > 100)
        .map(|(_, x, y)| (*x, *y))
        .expect("ElectroAnomaly root");
    let (child_x, child_y) = texts
        .iter()
        .find(|(t, x, _)| t == "↳ ElectroAnomaly_Dynamic" && *x > 100)
        .map(|(_, x, y)| (*x, *y))
        .unwrap_or_else(|| panic!("child '↳ ElectroAnomaly_Dynamic' missing.\n{dump}"));
    // The child name is indented one hierarchy level right of the parent
    // (one "↳" marker, depth encoded by the offset — see HIER_INDENT).
    let ind = crate::ui::metrics::HIER_INDENT.round() as i32;
    assert!(
        (child_x - (root_x + ind)).abs() <= 1,
        "child name must be indented 1×HIER_INDENT right of the parent\n(child_x={child_x} root_x={root_x})"
    );
    assert!(
        child_y > root_y,
        "child not below parent (child_y={child_y} root_y={root_y})"
    );

    // The parent's Radius value is on the parent row (editable cell).
    let has_radius = texts
        .iter()
        .any(|(t, x, y)| t == "200.0" && *x > 100 && (*y - root_y).abs() <= 10);
    assert!(
        has_radius,
        "ElectroAnomaly Radius (200.0) not on its row.\n{dump}"
    );

    // The PSY block (no editable field) still yields a row.
    let psy = texts
        .iter()
        .any(|(t, x, _)| t == "PSYControllerAnomaly" && *x > 100);
    assert!(psy, "PSYControllerAnomaly row missing.\n{dump}");
}

/// ARTIFACTS page (world artifact spawners): the config-only category
/// renders its 10 columns and one row per block × rank, the rank label
/// distinguishing the rows of the same block.
#[test]
fn artifact_spawner_page_renders() {
    let mut ws = test_workspace();
    let section_idx = ws
        .config()
        .sections()
        .iter()
        .position(|s| s == "ARTIFACTS")
        .expect("ARTIFACTS section in the sidebar dropdown");
    let cat = ws
        .config()
        .category_id("ArtifactSpawner")
        .expect("ArtifactSpawner category");
    let mut st = EditorState::default();
    st.view.section = section_idx;
    st.view.category = cat;
    // Both fixture spawners, expanded.
    for id in ws.entry_ids() {
        if ws.in_category(id, cat) {
            st.view.expanded.insert(ws.entry(id).block);
        }
    }

    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();
    let texts: Vec<(String, i32, i32)> = out
        .shapes
        .iter()
        .filter_map(|cs| match &cs.shape {
            egui::Shape::Text(t) => Some((
                t.galley.text().to_string(),
                t.pos.x.round() as i32,
                t.pos.y.round() as i32,
            )),
            _ => None,
        })
        .collect();
    let dump = texts
        .iter()
        .map(|(s, x, y)| format!("{x:4},{y:4}  {s}"))
        .collect::<Vec<_>>()
        .join("\n");

    // Column headers: the config `long` names, uppercased. The columns are
    // sized on these labels (no truncation); the full `long` name lives in
    // the field modal.
    for h in [
        "COUNT", "RADIUS", "MIN COOL", "MAX COOL", "BASE", "BONUS", "COMMON", "EPIC",
    ] {
        assert!(
            texts.iter().any(|(s, _, _)| s == h),
            "header {h} missing.\n{dump}"
        );
    }

    // One row per block × rank: the spawner name is painted once per rank
    // (in the body, below the header), the rank label next to it.
    for name in ["Empty", "ChunkMeatArtifactSpawner"] {
        let rows = texts
            .iter()
            .filter(|(t, x, y)| t == name && *x > 100 && *y > 100 && *y < 760)
            .count();
        assert!(rows >= 1, "no row for {name}.\n{dump}");
    }
    for rank in ["Newbie", "Veteran"] {
        assert!(
            texts.iter().any(|(t, x, _)| t == rank && *x > 100),
            "rank label {rank} missing.\n{dump}"
        );
    }

    // A fixture value in a cell (ChunkMeat Newbie Count = 4).
    assert!(
        texts.iter().any(|(t, _, _)| t == "4"),
        "Count value missing.\n{dump}"
    );
}

#[test]
fn weapon_rows_render() {
    let mut ws = test_workspace();
    let section_idx = ws
        .config()
        .sections()
        .iter()
        .position(|s| s == "STATS")
        .expect("STATS section present");
    let mut st = EditorState::default();
    st.view.section = section_idx;
    // Each weapon family: the sidebar row is painted (short label) and the
    // table shows the weapon name of the fixture block.
    for (cat, short, name) in [
        ("WeaponFire", "Firing", "GunTestPM_HG"),
        ("WeaponJams", "Jamming", "TemplateWeapon"),
        ("NpcWeaponCombat", "NPC Combat", "GunTestPM_HG_NPC"),
    ] {
        st.view.category = ws
            .config()
            .category_id(cat)
            .unwrap_or_else(|| panic!("{cat} category present"));
        let ctx = egui::Context::default();
        crate::ui_kit::apply_theme(&ctx);
        ctx.begin_pass(raw_1440());
        show_editor(&ctx, &mut ws, &mut st);
        let out = ctx.end_pass();
        let texts: Vec<String> = out
            .shapes
            .iter()
            .filter_map(|cs| match &cs.shape {
                egui::Shape::Text(t) => Some(t.galley.text().to_string()),
                _ => None,
            })
            .collect();
        let dump = texts.join("\n");
        assert!(
            texts.iter().any(|t| t == short),
            "{cat}: sidebar row '{short}' missing.\n{dump}"
        );
        // Inheriting blocks (refkey) are painted with the `↳` prefix, and
        // long names are truncated with `…` in the narrow name column —
        // match on a prefix.
        let prefix: String = name.chars().take(12).collect();
        assert!(
            texts.iter().any(|t| t.contains(&prefix)),
            "{cat}: row starting with '{prefix}' missing.\n{dump}"
        );
    }
    // Weapon GeneralSetup is 4 levels deep in the fixture (TemplateWeapon
    // → family template → gun → gun variant, like the real file): the
    // hierarchy is walked recursively, ONE "↳" marker per child (whatever
    // the depth) and the name indented by `depth × HIER_INDENT` to the
    // right — the offset, not a stack of arrows, encodes the depth.
    // Tints: each family (depth ≤ 1, with children) gets its OWN color
    // from GROUP_TINTS; guns and deeper variants INHERIT the family tint —
    // a depth-2 parent opens no new color.
    // `st` is still on the last loop category (NpcWeaponCombat) — switch
    // back to WeaponFire for the hierarchy assertions.
    st.view.category = ws
        .config()
        .category_id("WeaponFire")
        .unwrap_or_else(|| panic!("WeaponFire category present"));
    let (labels, rects) = {
        let ctx = egui::Context::default();
        crate::ui_kit::apply_theme(&ctx);
        ctx.begin_pass(raw_1440());
        show_editor(&ctx, &mut ws, &mut st);
        let out = ctx.end_pass();
        let labels = out
            .shapes
            .iter()
            .filter_map(|cs| match &cs.shape {
                egui::Shape::Text(t) => Some((t.pos, t.galley.text().to_string())),
                _ => None,
            })
            .collect::<Vec<_>>();
        let rects = out
            .shapes
            .iter()
            .filter_map(|cs| match &cs.shape {
                egui::Shape::Rect(r) => Some((r.rect, r.fill)),
                _ => None,
            })
            .collect::<Vec<_>>();
        (labels, rects)
    };
    let dump: Vec<String> = labels.iter().map(|(_, t)| t.as_str().into()).collect();
    let find =
        |s: &str| -> Option<(egui::Pos2, String)> { labels.iter().find(|(_, t)| t == s).cloned() };
    let root = find("TemplateWeapon").expect("TemplateWeapon row");
    // The exact text carries a SINGLE arrow (a stacked-arrow regression
    // would paint "↳ ↳ …" and miss this lookup).
    let fam1 = find("↳ TemplateTestPistol").unwrap_or_else(|| {
        panic!(
            "family template row missing (level 2).\n{}",
            dump.join("\n")
        )
    });
    let fam2 = find("↳ TemplateTestRifle")
        .unwrap_or_else(|| panic!("second family row missing.\n{}", dump.join("\n")));
    let gun1 = find("↳ GunTestPM_HG")
        .unwrap_or_else(|| panic!("gun row missing (level 3).\n{}", dump.join("\n")));
    let gun2 = find("↳ GunTestRifle_ST")
        .unwrap_or_else(|| panic!("second family gun missing.\n{}", dump.join("\n")));
    let scope1 = find("↳ GunTestPM_HG_Scope")
        .unwrap_or_else(|| panic!("gun variant row missing (level 4).\n{}", dump.join("\n")));
    let ind = crate::ui::metrics::HIER_INDENT;
    let near = |a: f32, b: f32| (a - b).abs() < 1.0;
    assert!(
        near(fam1.0.x, root.0.x + ind),
        "family template must be indented 1×HIER_INDENT: {} vs {}",
        fam1.0.x,
        root.0.x + ind
    );
    assert!(
        near(gun1.0.x, root.0.x + 2.0 * ind),
        "gun must be indented 2×HIER_INDENT: {} vs {}",
        gun1.0.x,
        root.0.x + 2.0 * ind
    );
    assert!(
        near(fam2.0.x, root.0.x + ind),
        "second family must be indented 1×HIER_INDENT: {} vs {}",
        fam2.0.x,
        root.0.x + ind
    );
    assert!(
        near(gun2.0.x, root.0.x + 2.0 * ind),
        "second family gun must be indented 2×HIER_INDENT: {} vs {}",
        gun2.0.x,
        root.0.x + 2.0 * ind
    );
    assert!(
        near(scope1.0.x, root.0.x + 3.0 * ind),
        "gun variant must be indented 3×HIER_INDENT: {} vs {}",
        scope1.0.x,
        root.0.x + 3.0 * ind
    );
    // Tints: row_frame paints the group tint as a full-row filled rect;
    // match it to the row by the name position.
    let palette = crate::ui_kit::c::GROUP_TINTS;
    let tint_of = |name: &str| -> Option<egui::Color32> {
        let (p, _) = labels.iter().find(|(_, t)| t == name)?;
        rects
            .iter()
            .find(|(r, f)| r.contains(*p) && palette.contains(f))
            .map(|(_, f)| *f)
    };
    let root_t = tint_of("TemplateWeapon").expect("root tint");
    let fam1_t = tint_of("↳ TemplateTestPistol").expect("fam1 tint");
    let gun1_t = tint_of("↳ GunTestPM_HG").expect("gun1 tint");
    let scope1_t = tint_of("↳ GunTestPM_HG_Scope").expect("scope1 tint");
    let fam2_t = tint_of("↳ TemplateTestRifle").expect("fam2 tint");
    let gun2_t = tint_of("↳ GunTestRifle_ST").expect("gun2 tint");
    assert_eq!(gun1_t, fam1_t, "gun inherits the family tint");
    assert_eq!(
        scope1_t, fam1_t,
        "depth-3 variant inherits the FAMILY tint (a depth-2 parent opens no new color)"
    );
    assert_eq!(
        gun2_t, fam2_t,
        "second family gun inherits its own family tint"
    );
    assert_ne!(fam1_t, fam2_t, "each family must get its own color");
    assert_ne!(root_t, fam1_t, "root template keeps its own color");
    assert_ne!(root_t, fam2_t, "root template keeps its own color");
    // NOTE: icon presence is NOT asserted here — a bare headless
    // `egui::Context` never emits textured meshes (the texture loads,
    // but the image shape is only produced by a real backend), so it
    // can only be verified in the app itself.
}

/// A hierarchy child whose parent is excluded by the search is still
/// painted (at the top level, no "↳"): a search for a gun name matches no
/// template, so without the fallback the table would be empty.
#[test]
fn search_shows_child_with_hidden_parent() {
    let mut ws = test_workspace();
    let section_idx = ws
        .config()
        .sections()
        .iter()
        .position(|s| s == "STATS")
        .expect("STATS section present");
    let mut st = EditorState::default();
    st.view.section = section_idx;
    st.view.category = ws
        .config()
        .category_id("WeaponFire")
        .unwrap_or_else(|| panic!("WeaponFire category present"));
    st.view.search = "GunTestPM".into();
    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();
    let texts: Vec<String> = out
        .shapes
        .iter()
        .filter_map(|cs| match &cs.shape {
            egui::Shape::Text(t) => Some(t.galley.text().to_string()),
            _ => None,
        })
        .collect();
    let dump = texts.join("\n");
    assert!(
        texts.iter().any(|t| t.contains("GunTestPM_HG")),
        "search 'GunTestPM' must show the gun row (parent templates hidden).\n{dump}"
    );
}

/// Value columns are sized on their header labels (the config `long`
/// names): the labels are painted in full — never truncated with an
/// ellipsis — and the content of a wide category extends beyond the
/// window (horizontal scroll, option A of the column-width design).
#[test]
fn column_headers_are_not_truncated() {
    let mut ws = test_workspace();
    let section_idx = ws
        .config()
        .sections()
        .iter()
        .position(|s| s == "STATS")
        .expect("STATS section present");
    let mut st = EditorState::default();
    st.view.section = section_idx;

    fn headers(ctx: &egui::Context, ws: &mut Workspace, st: &mut EditorState) -> Vec<String> {
        ctx.begin_pass(raw_1440());
        show_editor(ctx, ws, st);
        let out = ctx.end_pass();
        out.shapes
            .iter()
            .filter_map(|cs| match &cs.shape {
                egui::Shape::Text(t) => Some(t.galley.text().to_string()),
                _ => None,
            })
            .collect()
    }

    // WeaponFire (6 columns): the whole value zone fits in the 1440 px
    // window, so every header label is painted in full.
    st.view.category = ws.config().category_id("WeaponFire").unwrap();
    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);
    let texts = headers(&ctx, &mut ws, &mut st);
    let dump = texts.join("\n");
    for h in ["FIRE INTERVAL", "RECOIL INTERVAL", "FIRE QUEUE COUNT"] {
        assert!(
            texts.iter().any(|t| t == h),
            "header '{h}' missing.\n{dump}"
        );
    }

    // NpcWeaponCombat (12 columns): the zone is wider than the window —
    // the table scrolls horizontally, but the visible labels are still
    // painted in full.
    st.view.category = ws.config().category_id("NpcWeaponCombat").unwrap();
    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);
    let texts = headers(&ctx, &mut ws, &mut st);
    let dump = texts.join("\n");
    for h in ["BASE DAMAGE", "CHANCE BLEEDING PER SHOT"] {
        assert!(
            texts.iter().any(|t| t == h),
            "header '{h}' missing.\n{dump}"
        );
    }
}

/// The horizontal scroll moves the row names and the editable cells
/// TOGETHER (regression: the cells used to be positioned in fixed body
/// coordinates, so only the names and the header labels moved).
#[test]
fn cells_scroll_with_the_rows() {
    let mut ws = test_workspace();
    let section_idx = ws
        .config()
        .sections()
        .iter()
        .position(|s| s == "STATS")
        .expect("STATS section present");
    let mut st = EditorState::default();
    st.view.section = section_idx;
    // A wide category (12 columns) so the horizontal scroll is active.
    st.view.category = ws.config().category_id("NpcWeaponCombat").unwrap();

    fn is_cell_value(t: &str) -> bool {
        !t.is_empty()
            && t.chars()
                .all(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '%'))
    }

    /// One pass; returns (cell value anchors, row name anchors) and the
    /// position of the "BASE DAMAGE" header label (header lockstep check).
    fn pass(
        ctx: &egui::Context,
        ws: &mut Workspace,
        st: &mut EditorState,
        wheel_x: f32,
    ) -> (
        Vec<(egui::Pos2, String)>,
        Vec<egui::Pos2>,
        Option<egui::Pos2>,
    ) {
        let mut raw = raw_1440();
        // The wheel event only scrolls the area under the pointer: park
        // it ON a row (the first item row is painted at y ≈ 136).
        raw.events.push(egui::Event::PointerButton {
            pos: pos2(700.0, 136.0),
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        });
        if wheel_x != 0.0 {
            raw.events.push(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(wheel_x, 0.0),
                modifiers: Default::default(),
            });
        }
        ctx.begin_pass(raw);
        show_editor(ctx, ws, st);
        let out = ctx.end_pass();
        let (mut cells, mut names, mut hdr) = (Vec::new(), Vec::new(), None);
        for cs in &out.shapes {
            if let egui::Shape::Text(t) = &cs.shape {
                let text = t.galley.text();
                if is_cell_value(text) && t.pos.x >= 250.0 {
                    // `x >= 250` keeps the table cells and skips the static
                    // digit labels of the sidebar (which never scroll).
                    cells.push((t.pos, text.to_string()));
                } else if text == "BASE DAMAGE" {
                    hdr = Some(t.pos);
                } else if text.contains("GunTestPM_HG") {
                    names.push(t.pos);
                }
            }
        }
        (cells, names, hdr)
    }

    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);
    // Baseline (offset 0): capture the anchors.
    let (cells_a, names_a, hdr_a) = pass(&ctx, &mut ws, &mut st, 0.0);
    assert!(!cells_a.is_empty(), "no cell values painted (baseline)");
    assert!(!names_a.is_empty(), "no row name painted (baseline)");
    let hdr_a = hdr_a.expect("'BASE DAMAGE' header label painted (baseline)");

    // Scroll: the scroll area applies the offset to its content one frame
    // AFTER the wheel event, so send the wheel for several passes until the
    // row name has clearly moved to the left.
    type Snapshot = (
        Vec<(egui::Pos2, String)>,
        Vec<egui::Pos2>,
        Option<egui::Pos2>,
    );
    let mut name_dx = 0.0_f32;
    let mut snapshots: Vec<Snapshot> = Vec::new();
    for _ in 0..60 {
        // The wheel delta is smoothed over time: give it a frame of dt.
        std::thread::sleep(std::time::Duration::from_millis(120));
        // Negative x: the content moves left, revealing the right columns.
        let (c, n, h) = pass(&ctx, &mut ws, &mut st, -80.0);
        if let Some(b) = n.first() {
            name_dx = names_a[0].x - b.x;
        }
        snapshots.push((c, n, h));
        if name_dx > 40.0 {
            break;
        }
    }
    // The last snapshot: the one where the scroll has clearly taken effect.
    let (cells_b, _names_b, hdr_b) = snapshots
        .pop()
        .expect("at least one scroll pass was painted");
    assert!(
        name_dx > 40.0,
        "the horizontal scroll had no effect: name_dx={name_dx}"
    );

    // Every baseline cell is still painted (possibly off-screen), in the same
    // order: each must have moved by exactly the same amount as the row name.
    assert_eq!(cells_b.len(), cells_a.len(), "cells lost by scrolling");
    for ((a, atext), (b, _)) in cells_a.iter().zip(cells_b.iter()) {
        assert!(
            (a.x - b.x - name_dx).abs() < 2.0,
            "cell {atext:?} moved by {} but the row name moved by {name_dx}",
            a.x - b.x
        );
    }

    // Header lockstep: the column labels move by exactly the same amount
    // as the rows (regression: the header used the POST-update scroll
    // offset and led the rows by one frame's wheel increment).
    let hb = hdr_b.expect("'BASE DAMAGE' header label painted (after scroll)");
    assert!(
        (hdr_a.x - hb.x - name_dx).abs() < 2.0,
        "header moved by {} but the row name moved by {name_dx}",
        hdr_a.x - hb.x
    );

    // The horizontal scrollbar is pinned to the bottom of the row window
    // (body bottom = window 810 - status 32 - batch bar 52 = 726), not
    // under the last row — the fixture category has a single row near
    // y = 136, so a content-sized bar would sit around y = 160.
    // (Regression: the default `auto_shrink` of the ScrollArea shrank it
    // to the rows' height and anchored the bar to the shrunken bottom.)
    {
        let mut raw = raw_1440();
        raw.events.push(egui::Event::PointerButton {
            pos: pos2(700.0, 136.0),
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        });
        ctx.begin_pass(raw);
        show_editor(&ctx, &mut ws, &mut st);
        let out = ctx.end_pass();
        let body_bottom = 810.0f32 - metrics::STATUS_H - metrics::BATCH_H;
        let bar = out
            .shapes
            .iter()
            .filter_map(|cs| match &cs.shape {
                // The floating bar is a 2 px track + handle (width >= 100).
                egui::Shape::Rect(r) if r.rect.height() <= 4.0 && r.rect.width() >= 100.0 => {
                    Some(r.rect)
                }
                _ => None,
            })
            .any(|r| (r.bottom() - body_bottom).abs() < 2.0);
        assert!(
            bar,
            "horizontal scrollbar not pinned to the bottom of the row window (y={body_bottom})"
        );
    }
}
