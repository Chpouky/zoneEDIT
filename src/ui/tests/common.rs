//! Shared helpers (workspace, painted frames, clicks) — headless UI tests (egui without GPU).

use crate::domain::FieldId;
use crate::ui::*;
use egui::Pos2;

/// Test workspace on the synthetic fixtures (`tests/fixtures`):
/// independent of the game's files.
pub(crate) fn test_workspace() -> Workspace {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    crate::io::load::load(&root).expect("loading of the fixtures")
}

/// Commit with the same behaviour as the UI (st.msg set in case of refusal).
pub(crate) fn do_commit(ws: &mut Workspace, st: &mut EditorState, key: FieldId, value: &str) {
    match crate::ui::ops::commit(ws, key, value) {
        Ok(()) => {}
        Err(m) => st.msg = Some((false, m)),
    }
}

/// A frame of the editor without GPU, with the same theme as the app.
pub(crate) fn painted_frame() -> (egui::FullOutput, Workspace) {
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
    (ctx.end_pass(), ws)
}

/// Runs a frame of the editor without GPU and lists the painted text.
pub(crate) fn painted_texts() -> (Vec<(String, i32, i32)>, String) {
    let (out, ws) = painted_frame();
    let mut texts = Vec::new();
    for cs in &out.shapes {
        if let egui::Shape::Text(t) = &cs.shape {
            texts.push((
                t.galley.text().to_string(),
                t.pos.x.round() as i32,
                t.pos.y.round() as i32,
            ));
        }
    }
    // First entry of the MedCrate category (config id).
    let med = ws
        .config()
        .category_id("MedCrate")
        .expect("MedCrate category");
    let first_block = ws
        .entry_ids()
        .find(|&id| ws.in_category(id, med))
        .map(|id| ws.entry(id).block);
    let n = texts.len();
    (texts, format!("first_block={first_block:?} n_texts={n}"))
}

// ---- bug repro: cell editing (click + typing) -------------------
pub(crate) fn raw_1440() -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1440.0, 810.0))),
        focused: true,
        ..Default::default()
    }
}

/// Copies the fixtures into a temporary directory (the tests that
/// WRITE — preset save/delete — must not dirty `tests/fixtures`).
/// The `TempDir` is cleaned up automatically at the end of the test.
pub(crate) fn temp_root(tag: &str) -> tempfile::TempDir {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let dst = tempfile::Builder::new()
        .prefix(format!("zoneedit_{tag}_").as_str())
        .tempdir()
        .expect("tempdir");
    fn copy_rec(s: &std::path::Path, d: &std::path::Path) -> std::io::Result<()> {
        std::fs::create_dir_all(d)?;
        for e in std::fs::read_dir(s)? {
            let e = e?;
            if e.file_type()?.is_dir() {
                copy_rec(&e.path(), &d.join(e.file_name()))?;
            } else {
                std::fs::copy(e.path(), d.join(e.file_name()))?;
            }
        }
        Ok(())
    }
    copy_rec(&src, dst.path()).expect("fixtures copy");
    dst
}

pub(crate) fn click_at(ctx: &egui::Context, pos: Pos2) {
    let mut raw = raw_1440();
    raw.events.push(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    });
    raw.events.push(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    });
    ctx.begin_pass(raw);
}

/// Helper: a full primary click at `p`.
pub(crate) fn click(raw: &mut egui::RawInput, p: Pos2) {
    raw.events.push(egui::Event::PointerButton {
        pos: p,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Default::default(),
    });
    raw.events.push(egui::Event::PointerButton {
        pos: p,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Default::default(),
    });
}
