//! Editor operations (commit, reset, presets, reloading).

use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use egui::{vec2, Id, Pos2, Rect, Sense, Stroke, Ui};

use crate::domain::{Entry, FieldId, Workspace};
use crate::io;
use crate::io::presets::Preset;
use crate::ui::state::{Action, ConfirmAction, EditorState, Modal};
use crate::ui_kit::c;

/// Choix de preset (0 = vanilla) : demande confirmation si des modifs pendent.
pub(crate) fn apply_preset_choice(ws: &mut Workspace, st: &mut EditorState, choice: usize) {
    if choice == st.presets.choice {
        st.presets.open = false;
        return;
    }
    if ws.dirty_count() > 0 {
        st.modal = Some(Modal::Confirm(ConfirmAction::ApplyPreset(choice)));
        st.presets.open = false;
        return;
    }
    if choice == 0 {
        ws.reset_all();
    } else if let Some(p) = st.presets.list.get(choice - 1) {
        let (ok, msg) = apply_preset(ws, p);
        st.msg = Some((ok, msg));
    }
    st.presets.choice = choice;
    st.presets.open = false;
}

/// Name of the target of a delegation.
pub(crate) fn delegation_target(e: &Entry) -> &str {
    e.sid.as_str()
}

// ===================== TABLE =====================

/// Commit of a cell value: `Ok(())` if accepted, `Err(message)`
/// if rejected (chance outside [0, 100 %], Min > Max pair, …). All the
/// normalization and the checking live in the domain.
pub(crate) fn commit(ws: &mut Workspace, id: FieldId, value: &str) -> Result<(), String> {
    ws.commit_input(id, value).map_err(|e| e.to_string())
}

// ===================== ACTIONS =====================

/// Recharge le workspace depuis le disque. Renvoie les presets du root.
pub(crate) fn do_reload(ws: &mut Workspace) -> Result<Vec<Preset>, String> {
    let root = ws.root.clone();
    let chosen = ws.chosen_dir.clone();
    let w = io::load::load(&root).map_err(|e| e.to_string())?;
    let (presets, _errs) = io::presets::load_all(&root, w.config());
    *ws = w;
    ws.chosen_dir = chosen;
    Ok(presets)
}

/// Applies the result of a `do_reload` to the state (views, presets, message).
pub(crate) fn finish_reload(st: &mut EditorState, res: Result<Vec<Preset>, String>) {
    match res {
        Ok(presets) => {
            st.reset_view();
            st.presets.list = presets;
            st.msg = Some((true, "Files reloaded from disk.".into()));
        }
        Err(e) => st.msg = Some((false, e)),
    }
}

/// Loads a project (given root). Returns (display name, presets).
pub(crate) fn load_project(
    ws: &mut Workspace,
    root: &std::path::Path,
    chosen: &std::path::Path,
) -> Result<(String, Vec<Preset>), String> {
    let w = io::load::load(root).map_err(|e| e.to_string())?;
    // The display name is that of the folder CHOSEN in the explorer
    // (not that of the resolved root, e.g. "GameLite").
    let name = chosen
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let (presets, _errs) = io::presets::load_all(&w.root, w.config());
    *ws = w;
    ws.chosen_dir = chosen.to_path_buf();
    Ok((name, presets))
}

/// Applies the result of a `load_project` to the state and remembers the
/// chosen folder so the next launch opens it automatically.
pub(crate) fn finish_load(
    st: &mut EditorState,
    res: Result<(String, Vec<Preset>), String>,
    chosen: &std::path::Path,
) {
    match res {
        Ok((name, presets)) => {
            st.reset_view();
            st.presets.list = presets;
            st.msg = Some((true, format!("Loaded project {name}.")));
            io::appstate::save_last_root(chosen);
        }
        Err(e) => st.msg = Some((false, e)),
    }
}

/// Applies a preset to the workspace. Returns (success, message).
pub(crate) fn apply_preset(ws: &mut Workspace, p: &Preset) -> (bool, String) {
    ws.reset_all();
    let rep = io::presets::apply(ws, p);
    if p.version == 1 {
        (
            true,
            format!(
                "Preset \"{}\" (v1) applied by line number — {} field(s), no context check. Re-save it to upgrade to v3.",
                p.name, rep.applied,
            ),
        )
    } else {
        (
            rep.skipped == 0,
            format!(
                "Preset \"{}\": {} field(s) applied, {} ignored.",
                p.name, rep.applied, rep.skipped
            ),
        )
    }
}

/// Exports the mod (ZoneKit patch files) and sets the message.
///
/// The user picks the folder of an existing mod in the OS explorer: the
/// mod name is the name of that folder, and the patches are written under
/// its `GameData` folder (the mod mirrors the game tree under
/// `Content/GameLite/`, article 173).
pub(crate) fn export_now(ws: &mut Workspace, st: &mut EditorState) {
    let Some(dir) = rfd::FileDialog::new().pick_folder() else {
        return; // Picker cancelled: nothing to do.
    };
    let folder_name = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mod_name = io::export::sanitize_mod_name(&folder_name);
    let out_dir = mod_content_dir(&dir);
    match io::export::export(ws, &out_dir, &mod_name) {
        Ok(list) => {
            st.msg = Some((
                true,
                format!(
                    "Exported {} ZoneKit patch file(s) to {}.",
                    list.len(),
                    out_dir.display()
                ),
            ));
        }
        Err(e) => st.msg = Some((false, e.to_string())),
    }
}

/// Content root of the mod that the spec paths (`GameData/...`) are
/// relative to. The config file specs are relative to the game root and a
/// mod mirrors the game tree under `Content/GameLite/` (article 173), so
/// the patches end up in `<dir>/Content/GameLite/GameData/...`. The
/// existing structure is detected instead of being recreated:
///
/// 1. `<dir>/Content/GameLite` — a ZoneKit source mod root was picked;
/// 2. `<dir>` — it already carries a `GameData` folder;
/// 3. `<dir>/..` — the picked folder is the GameData folder itself;
/// 4. otherwise `<dir>/Content/GameLite` is created fresh.
fn mod_content_dir(dir: &Path) -> PathBuf {
    let zk = dir.join("Content").join("GameLite");
    if zk.is_dir() {
        return zk;
    }
    if dir.join("GameData").is_dir() {
        return dir.to_path_buf();
    }
    if dir.file_name().and_then(|n| n.to_str()) == Some("GameData")
        || dir.join("ItemGeneratorPrototypes.cfg").is_file()
    {
        if let Some(p) = dir.parent() {
            return p.to_path_buf();
        }
    }
    zk
}

/// Applies an action from the `pending_actions` queue.
pub(crate) fn apply_action(ws: &mut Workspace, st: &mut EditorState, a: Action) {
    match a {
        Action::Reload => {
            if ws.dirty_count() > 0 {
                st.modal = Some(Modal::Confirm(ConfirmAction::Reload));
            } else {
                finish_reload(st, do_reload(ws));
            }
        }
        Action::Export => export_now(ws, st),
        Action::ShowDiff => st.modal = Some(Modal::Diff),
        Action::ShowErrors => {
            // Opens/closes the panel ("N error(s)" button).
            st.modal = if matches!(st.modal, Some(Modal::Errors)) {
                None
            } else {
                Some(Modal::Errors)
            };
        }
        Action::ResetSelection => {
            reset_entries_from_keys(ws, &st.selection.batch);
        }
        Action::ResetAll => {
            ws.reset_all();
        }
        Action::Batch { idx, anchor } => {
            let n = ws.config().batch[idx]
                .inputs
                .len()
                .max(ws.config().batch[idx].fields.len());
            st.modal = Some(Modal::Batch {
                idx,
                inputs: vec![String::new(); n],
                anchor,
                meas_h: 0.0,
            });
        }
    }
}

/// Small circular button (↺ arrow) that restores the given fields to their
/// original value. Is only drawn if one of the fields is modified.
/// Returns `true` if the button was clicked (so as not to select the
/// line at the same time).
pub(crate) fn draw_row_reset(
    ui: &mut Ui,
    ws: &mut Workspace,
    st: &mut EditorState,
    keys: &[FieldId],
    center: Pos2,
) -> bool {
    if !keys.iter().any(|k| ws.field(*k).is_dirty()) {
        return false;
    }
    let rect = Rect::from_center_size(center, vec2(20.0, 20.0));
    // `ui.interact` (pas `allocate_rect`) : ne fait pas avancer le curseur,
    // otherwise the height of the following lines would be skewed.
    let id = Id::new(("row_reset", keys.first().copied()));
    let resp = ui.interact(rect, id, Sense::click());
    let p = ui.painter_at(rect);
    let hov = resp.hovered();
    p.circle_filled(center, 9.0, if hov { c::HOVER } else { c::INPUT_BG });
    let fg = if hov { c::MODIFIED_FG } else { c::TEXT_DIM };
    // Arc of ~300° (empty on the left), drawn in small segments.
    let r = 5.5_f32;
    let a0 = -2.6_f32;
    let a1 = 2.6_f32;
    let mut pts = Vec::new();
    const STEPS: usize = 24;
    for i in 0..=STEPS {
        let a = a0 + (a1 - a0) * (i as f32 / STEPS as f32);
        pts.push(center + vec2(a.cos(), a.sin()) * r);
    }
    p.line(pts, Stroke::new(1.5_f32, fg));
    // Arrowhead at the beginning of the arc, oriented tangentially.
    let rad = vec2(a0.cos(), a0.sin());
    let tan = vec2(-a0.sin(), a0.cos());
    let base = center + rad * r;
    p.add(egui::Shape::convex_polygon(
        vec![
            base + tan * 4.5,
            base + rad * 3.0 - tan,
            base - rad * 3.0 - tan,
        ],
        fg,
        Stroke::NONE,
    ));
    if resp.clicked() {
        // Closes the editor if a field of the line was open, otherwise its
        // buffer (stale) would be re-committed on top of the reset.
        if st.editing.cell.is_some_and(|k| keys.contains(&k)) {
            st.editing.cell = None;
            st.editing.text.clear();
        }
        ws.reset(keys.iter().copied());
        return true;
    }
    false
}

/// Resets to zero all the fields (item + group) of the entries whose
/// first field is in `keys`.
pub(crate) fn reset_entries_from_keys(ws: &mut Workspace, keys: &HashSet<FieldId>) {
    let entries: Vec<_> = keys
        .iter()
        .filter_map(|k| ws.owner(*k))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    ws.reset_entries(entries);
}
