//! menus_batch — presets menu + batch spawn % modal — headless UI tests (egui without GPU).

use crate::domain::FieldKind;
use crate::ui::*;
use egui::{pos2, Pos2};

use super::common::*;
/// Preset menu: typing the name then pressing Enter creates the preset (like the "+").
#[test]
fn preset_menu_enter_creates_preset() {
    let root = temp_root("preset_enter");
    let root = root.path();
    let mut ws = crate::io::load::load(root).expect("load");
    let mut st = EditorState::default();
    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);

    // Frame 1: locate the PRESET dropdown ("Vanilla", bottom batch bar).
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();
    let dd = out
        .shapes
        .iter()
        .find_map(|cs| {
            if let egui::Shape::Text(t) = &cs.shape {
                (t.galley.text() == "Vanilla" && t.pos.y > 700.0)
                    .then(|| t.pos + t.galley.size() / 2.0)
            } else {
                None
            }
        })
        .expect("PRESET dropdown not painted");

    // Frame 2: click on the dropdown → menu open. Note: the area
    // (egui::Area) is invisible on the opening frame (sizing pass) — its
    // content is only painted starting from the next frame.
    click_at(&ctx, dd);
    show_editor(&ctx, &mut ws, &mut st);
    let _ = ctx.end_pass();
    assert!(st.presets.open, "preset menu did not open");

    // Frame 3: no input → the menu content is painted.
    ctx.begin_pass(raw_1440());
    show_editor(&ctx, &mut ws, &mut st);
    let out = ctx.end_pass();

    // Frame 4: click on the "New Preset…" field (focus).
    let field = out
        .shapes
        .iter()
        .find_map(|cs| {
            if let egui::Shape::Text(t) = &cs.shape {
                (t.galley.text() == "New Preset…").then(|| t.pos + t.galley.size() / 2.0)
            } else {
                None
            }
        })
        .expect("New Preset field not painted");
    click_at(&ctx, field);
    show_editor(&ctx, &mut ws, &mut st);
    let _ = ctx.end_pass();

    // Frame 5: type "T1".
    let mut raw = raw_1440();
    raw.events.push(egui::Event::Text("T1".to_string()));
    ctx.begin_pass(raw);
    show_editor(&ctx, &mut ws, &mut st);
    let _ = ctx.end_pass();

    // Frame 6: Enter → creation (the TextEdit loses focus on this frame).
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

    assert!(
        st.presets.list.iter().any(|p| p.name == "T1"),
        "preset not created after Enter"
    );
    assert!(
        root.join("presets").join("t1.preset").exists(),
        "preset file missing"
    );
}

/// "Set Min/Max Spawn %…" button: the modal opens, the % input is
/// converted to the cfg format (`25` → `0.25f`), out of range → rejected.
#[test]
fn set_spawn_batch_button_applies_percent() {
    let mut ws = test_workspace_batch();
    let mut st = EditorState::default();
    st.view.category = ws
        .config()
        .category_id("MedStash")
        .expect("MedStash category");
    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);

    // Index of the "Set Min/Max Spawn %…" batch config.
    let batch_idx = ws
        .config()
        .batch
        .iter()
        .position(|b| b.label == "Set Min/Max Spawn %…")
        .expect("\"Set Min/Max Spawn %…\" batch in the config");

    // StashMedicine_Smart entry carrying MinSpawnChance, block expanded.
    let e = visible_entries(&ws, &st)
        .into_iter()
        .find(|e| {
            !e.is_delegation()
                && !e.fields.is_empty()
                && ws.block(e.block).name == "StashMedicine_Smart"
                && e.group_fields
                    .iter()
                    .any(|&k| ws.field(k).kind == FieldKind::MinSpawnChance)
        })
        .expect("MedStash entry with MinSpawnChance");
    st.view.expanded.insert(e.block);
    let first_key = e.fields[0];
    let min_key = e
        .group_fields
        .iter()
        .find(|&&k| ws.field(k).kind == FieldKind::MinSpawnChance)
        .copied()
        .unwrap();

    // One frame: optional click (pos) or typed text.
    let frame = |click_at: Option<Pos2>,
                 text: Option<&str>,
                 w: &mut Workspace,
                 s: &mut EditorState|
     -> egui::FullOutput {
        let mut r = raw_1440();
        if let Some(p) = click_at {
            click(&mut r, p);
        }
        if let Some(t) = text {
            r.events.push(egui::Event::Text(t.to_string()));
        }
        ctx.begin_pass(r);
        show_editor(&ctx, w, s);
        ctx.end_pass()
    };
    /// Position of a painted text (y > min_y): (x, y, h).
    fn text_pos(out: &egui::FullOutput, txt: &str, min_y: f32) -> (f32, f32, f32) {
        out.shapes
            .iter()
            .filter_map(|cs| {
                if let egui::Shape::Text(t) = &cs.shape {
                    if t.galley.text() == txt && t.pos.y > min_y {
                        Some((t.pos.x, t.pos.y, t.galley.size().y))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .next()
            .unwrap_or_else(|| panic!("text {txt:?} (y > {min_y}) not found"))
    }

    // Frame 1: locate the batch button + the row checkbox.
    let out = frame(None, None, &mut ws, &mut st);
    let (bx, by, bh) = text_pos(&out, "Set Min/Max Spawn %…", 700.0);
    let (nx, ny, nh) = text_pos(&out, &e.sid, 100.0);
    // checkbox left+CB_CENTER_X, name left+ITEM_NAME_X
    let cb = pos2(
        nx - (crate::ui::metrics::ITEM_NAME_X - crate::ui::metrics::CB_CENTER_X),
        ny + nh / 2.0,
    );
    let btn = pos2(bx + 5.0, by + bh / 2.0);

    // Frame 2: click checkbox → row selected.
    let _ = frame(Some(cb), None, &mut ws, &mut st);
    assert!(
        st.selection.batch.contains(&first_key),
        "row selected by the checkbox"
    );

    // Frame 3: click button → modal open.
    let _ = frame(Some(btn), None, &mut ws, &mut st);
    assert!(
        matches!(&st.modal, Some(Modal::Batch { idx, .. }) if *idx == batch_idx),
        "the spawn % modal is open"
    );

    // Frame 4: 1st frame of the modal (the window is only painted on the next frame).
    let _ = frame(None, None, &mut ws, &mut st);
    // Frame 5: locate the Min field (right of the "Min :" label) and click it.
    let out = frame(None, None, &mut ws, &mut st);
    let (mx, my, mh) = text_pos(&out, "Min :", 0.0);
    let _ = frame(Some(pos2(mx + 55.0, my + mh / 2.0)), None, &mut ws, &mut st);

    // Frame 6: type "25". Frame 7: Apply.
    let _ = frame(None, Some("25"), &mut ws, &mut st);
    assert!(
        matches!(&st.modal, Some(Modal::Batch { inputs, .. }) if inputs.first() == Some(&"25".to_string())),
        "the modal buffer receives the input"
    );
    let out = frame(None, None, &mut ws, &mut st);
    let (ax, ay, ah) = text_pos(&out, "Apply", 0.0);
    let _ = frame(Some(pos2(ax + 5.0, ay + ah / 2.0)), None, &mut ws, &mut st);
    assert!(st.modal.is_none(), "the modal is closed after Apply");
    assert_eq!(
        ws.field(min_key).current(),
        Some("0.25f"),
        "\"25\" must be stored as 0.25f"
    );
    assert!(
        st.msg.as_ref().map(|(ok, _)| *ok).unwrap_or(false),
        "success message expected"
    );

    // Out of range: reopen, type "150", Apply → rejected.
    let _ = frame(Some(btn), None, &mut ws, &mut st);
    assert!(
        matches!(&st.modal, Some(Modal::Batch { idx, .. }) if *idx == batch_idx),
        "modal reopened"
    );
    let _ = frame(None, None, &mut ws, &mut st); // 1st frame of the modal
    let out = frame(None, None, &mut ws, &mut st);
    let (mx, my, mh) = text_pos(&out, "Min :", 0.0);
    let _ = frame(Some(pos2(mx + 55.0, my + mh / 2.0)), None, &mut ws, &mut st);
    let _ = frame(None, Some("150"), &mut ws, &mut st);
    let out = frame(None, None, &mut ws, &mut st);
    let (ax, ay, ah) = text_pos(&out, "Apply", 0.0);
    let _ = frame(Some(pos2(ax + 5.0, ay + ah / 2.0)), None, &mut ws, &mut st);
    assert!(st.modal.is_none());
    assert_eq!(
        ws.field(min_key).current(),
        Some("0.25f"),
        "150% must be rejected (value unchanged)"
    );
    assert!(
        st.msg.as_ref().map(|(ok, _)| !*ok).unwrap_or(false),
        "error message expected for 150%"
    );
}

/// Topbar menu (Folder): it opens on the frame of the click, where
/// `any_click` is still true (release event). It must NOT close by
/// itself, and an outside click closes it.
/// (Regression: with the full-screen catcher, the next frame swallowed
/// the click; with `clicked_elsewhere` without excluding the anchor,
/// the menu closed on its own opening frame.)
#[test]
fn topbar_folder_menu_stays_open_and_closes_outside() {
    let mut ws = test_workspace();
    let mut st = EditorState::default();
    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);

    let frame = |at: Option<Pos2>, w: &mut Workspace, s: &mut EditorState| -> egui::FullOutput {
        let mut raw = raw_1440();
        if let Some(p) = at {
            click(&mut raw, p);
        }
        ctx.begin_pass(raw);
        show_editor(&ctx, w, s);
        ctx.end_pass()
    };
    /// Position of a painted text (unique): (x, y, h).
    fn text_pos(out: &egui::FullOutput, txt: &str) -> (f32, f32, f32) {
        out.shapes
            .iter()
            .find_map(|cs| {
                if let egui::Shape::Text(t) = &cs.shape {
                    (t.galley.text() == txt).then(|| (t.pos.x, t.pos.y, t.galley.size().y))
                } else {
                    None
                }
            })
            .unwrap_or_else(|| panic!("text {txt:?} not found"))
    }

    // Frame 1: locate the "Folder" label; the dropdown follows (6 px).
    let out = frame(None, &mut ws, &mut st);
    let (fx, fy, fh) = text_pos(&out, "Folder");
    let dd = pos2(fx + 60.0, fy + fh / 2.0);

    // Frame 2: click on the dropdown → open… and still open at the end
    // of the SAME frame (the opening click is on the anchor).
    let _ = frame(Some(dd), &mut ws, &mut st);
    assert!(
        matches!(st.modal, Some(Modal::Menu(TopMenu::Project))),
        "the Folder menu must stay open on the frame of the opening click"
    );

    // Frame 3: no input → still open.
    let _ = frame(None, &mut ws, &mut st);
    assert!(
        matches!(st.modal, Some(Modal::Menu(TopMenu::Project))),
        "the Folder menu must not close by itself"
    );

    // Frame 4: outside click (middle of the table) → closed.
    let _ = frame(Some(pos2(720.0, 400.0)), &mut ws, &mut st);
    assert!(st.modal.is_none(), "an outside click must close the menu");
}

/// Presets menu: same regression as the topbar menu — it must stay open
/// on the frame of the opening click and close on an outside click.
#[test]
fn preset_menu_stays_open_and_closes_outside() {
    let mut ws = test_workspace();
    let mut st = EditorState::default();
    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);

    let frame = |at: Option<Pos2>, w: &mut Workspace, s: &mut EditorState| -> egui::FullOutput {
        let mut raw = raw_1440();
        if let Some(p) = at {
            click(&mut raw, p);
        }
        ctx.begin_pass(raw);
        show_editor(&ctx, w, s);
        ctx.end_pass()
    };
    /// Position of a painted text (y > min_y): (x, y, h).
    fn text_pos(out: &egui::FullOutput, txt: &str, min_y: f32) -> (f32, f32, f32) {
        out.shapes
            .iter()
            .find_map(|cs| {
                if let egui::Shape::Text(t) = &cs.shape {
                    (t.galley.text() == txt && t.pos.y > min_y)
                        .then(|| (t.pos.x, t.pos.y, t.galley.size().y))
                } else {
                    None
                }
            })
            .unwrap_or_else(|| panic!("text {txt:?} (y > {min_y}) not found"))
    }

    // Frame 1: locate the presets dropdown ("Vanilla", bottom bar).
    let out = frame(None, &mut ws, &mut st);
    let (vx, vy, vh) = text_pos(&out, "Vanilla", 700.0);
    let dd = pos2(vx + 5.0, vy + vh / 2.0);

    // Frame 2: click → open… and still open at the end of the same frame.
    let _ = frame(Some(dd), &mut ws, &mut st);
    assert!(
        st.presets.open,
        "the presets menu must stay open on the frame of the click"
    );

    // Frame 3: no input → still open.
    let _ = frame(None, &mut ws, &mut st);
    assert!(st.presets.open, "the presets menu must not close by itself");

    // Frame 4: outside click → closed.
    let _ = frame(Some(pos2(720.0, 400.0)), &mut ws, &mut st);
    assert!(
        !st.presets.open,
        "an outside click must close the presets menu"
    );
}

/// Batch modal "Set Chance…": the popup is sized as snug as possible
/// (no excessive width) and its content (fields + buttons) is centered,
/// not stuck to the left edge.
#[test]
fn set_chance_modal_is_centered_and_snug() {
    let mut ws = test_workspace_batch();
    let mut st = EditorState::default();
    st.view.category = ws
        .config()
        .category_id("AmmoGenerator")
        .expect("AmmoGenerator category");
    let ctx = egui::Context::default();
    crate::ui_kit::apply_theme(&ctx);

    let batch_idx = ws
        .config()
        .batch
        .iter()
        .position(|b| b.label == "Set Chance…")
        .expect("batch Set Chance");

    // Non-delegation entry with a Chance field, block expanded.
    let e = visible_entries(&ws, &st)
        .into_iter()
        .find(|e| {
            !e.is_delegation()
                && (e
                    .fields
                    .iter()
                    .any(|&k| ws.field(k).kind == FieldKind::Chance)
                    || e.group_fields
                        .iter()
                        .any(|&k| ws.field(k).kind == FieldKind::Chance))
        })
        .expect("entry with Chance field");
    st.view.expanded.insert(e.block);
    let first_key = e.fields[0];
    let (nx, ny, nh) = {
        let out0 = {
            let r = raw_1440();
            ctx.begin_pass(r);
            show_editor(&ctx, &mut ws, &mut st);
            ctx.end_pass()
        };
        out0.shapes
            .iter()
            .find_map(|cs| {
                if let egui::Shape::Text(t) = &cs.shape {
                    (t.galley.text() == e.sid).then(|| (t.pos.x, t.pos.y, t.galley.size().y))
                } else {
                    None
                }
            })
            .unwrap()
    };
    // checkbox left+CB_CENTER_X, name left+ITEM_NAME_X
    let cb = pos2(
        nx - (crate::ui::metrics::ITEM_NAME_X - crate::ui::metrics::CB_CENTER_X),
        ny + nh / 2.0,
    );

    let frame = |at: Option<Pos2>, w: &mut Workspace, s: &mut EditorState| -> egui::FullOutput {
        let mut r = raw_1440();
        if let Some(p) = at {
            click(&mut r, p);
        }
        ctx.begin_pass(r);
        show_editor(&ctx, w, s);
        ctx.end_pass()
    };

    let _ = frame(Some(cb), &mut ws, &mut st); // selects the row
    assert!(st.selection.batch.contains(&first_key));

    // Locate the "Set Chance…" button and click it.
    let out = frame(None, &mut ws, &mut st);
    let (bx, by, bh) = out
        .shapes
        .iter()
        .find_map(|cs| {
            if let egui::Shape::Text(t) = &cs.shape {
                (t.galley.text() == "Set Chance…" && t.pos.y > 700.0)
                    .then(|| (t.pos.x, t.pos.y, t.galley.size().y))
            } else {
                None
            }
        })
        .unwrap();
    let _ = frame(Some(pos2(bx + 5.0, by + bh / 2.0)), &mut ws, &mut st);
    assert!(
        matches!(&st.modal, Some(Modal::Batch { idx, inputs, .. })
            if *idx == batch_idx && inputs.len() == 1),
        "Set Chance modal open (1 field)"
    );

    let _ = frame(None, &mut ws, &mut st); // 1st frame of the modal (sizing)
    let out = frame(None, &mut ws, &mut st); // content painted

    // Center of the "Apply" button (to locate the popup frame).
    let (ax, ay, aw, ah) = out
        .shapes
        .iter()
        .find_map(|cs| {
            if let egui::Shape::Text(t) = &cs.shape {
                (t.galley.text() == "Apply")
                    .then(|| (t.pos.x, t.pos.y, t.galley.size().x, t.galley.size().y))
            } else {
                None
            }
        })
        .expect("Apply button painted");
    let p = pos2(ax + aw / 2.0, ay + ah / 2.0);

    // Popup frame = largest "popup-like" rect (reasonable size) that
    // contains the Apply button (avoids the full-screen background rects).
    let fr = out
        .shapes
        .iter()
        .filter_map(|cs| match &cs.shape {
            egui::Shape::Rect(r)
                if r.rect.contains(p) && r.rect.width() < 400.0 && r.rect.height() < 300.0 =>
            {
                Some(r.rect)
            }
            _ => None,
        })
        .max_by(|a, b| a.area().partial_cmp(&b.area()).unwrap())
        .expect("popup frame");

    // 1) Snug width (the old fixed width 240 was excessive).
    assert!(
        (130.0..220.0).contains(&fr.width()),
        "popup width excessive or too narrow: {:.0}",
        fr.width()
    );

    // 2) The "Chance :" label is not stuck to the left edge (centered).
    let (lx, _, _) = out
        .shapes
        .iter()
        .find_map(|cs| {
            if let egui::Shape::Text(t) = &cs.shape {
                (t.galley.text() == "Chance :").then(|| (t.pos.x, t.pos.y, t.galley.size().y))
            } else {
                None
            }
        })
        .expect("\"Chance :\" label painted");
    assert!(
        lx - fr.min.x > 18.0,
        "label stuck to the left edge (not centered): margin {:.0}",
        lx - fr.min.x
    );

    // 3) The button group is centered in the popup.
    // Left edge of the "Cancel" button (text - padding 12).
    let cancel_x = out
        .shapes
        .iter()
        .find_map(|cs| {
            if let egui::Shape::Text(t) = &cs.shape {
                (t.galley.text() == "Cancel").then_some(t.pos.x)
            } else {
                None
            }
        })
        .expect("Cancel button painted");
    // Right edge of the "Apply" button (end of text + padding 12).
    let apply_right = ax + aw + 12.0;
    let group_center = ((cancel_x - 12.0) + apply_right) / 2.0;
    let frame_center = fr.center().x;
    assert!(
        (group_center - frame_center).abs() < 14.0,
        "buttons not centered: center {group_center:.0} vs {frame_center:.0}"
    );
}
