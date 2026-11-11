//! Dropdown menus: topbar (project / file) and presets (upward).
//!
//! ALL the anchored popups of the app go through the same mold
//! `anchored_popup` (widgets.rs): `Area` in `fixed_pos` + `menu_frame`,
//! unique design. The outside-click closing is detected by the mold
//! itself (`PopupResult::clicked_outside`), without a full-screen catcher.

use egui::{pos2, vec2, Align2, FontId, Rect, Sense};

use crate::domain::Workspace;
use crate::io::{presets, resolve_root};
use crate::ui::metrics::{FONT_BODY, FONT_SMALL};
use crate::ui::ops::{apply_preset_choice, finish_load, load_project};
use crate::ui::state::{ConfirmAction, EditorState, Modal, TopMenu};
use crate::ui::widgets::{anchored_popup, draw_trash, menu_row, PopupSide};
use crate::ui_kit::{c, hline, primary_button};

/// Dropdown menu of the topbar (Folder / File), anchored under the dropdown.
pub(crate) fn draw_topbar_menu(
    ctx: &egui::Context,
    menu: &TopMenu,
    anchor: Rect,
    ws: &mut Workspace,
    st: &mut EditorState,
) {
    let res = anchored_popup(ctx, "top_menu", anchor, PopupSide::Below, 0.0, |ui| {
        ui.set_width(anchor.width().max(220.0));
        match menu {
            TopMenu::Project => {
                let projects = st.projects_for(ws);
                for p in &projects {
                    let label = p.file_name().and_then(|s| s.to_str()).unwrap_or("?");
                    if menu_row(ui, label, p.as_path() == ws.root.as_path()) {
                        st.modal = None;
                        if p.as_path() != ws.root.as_path() {
                            if ws.dirty_count() > 0 {
                                st.modal =
                                    Some(Modal::Confirm(ConfirmAction::LoadProject(p.clone())));
                            } else {
                                finish_load(st, load_project(ws, p, p), p);
                            }
                        }
                    }
                }
                hline(ui);
                if menu_row(ui, "Open folder…", false) {
                    // Native OS folder picker (rfd):
                    // no more path to type by hand. The CHOSEN folder
                    // is kept (displayed name), the root is resolved at
                    // load time.
                    if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                        if ws.dirty_count() > 0 {
                            st.modal = Some(Modal::Confirm(ConfirmAction::OpenFolder(dir)));
                        } else {
                            let root =
                                resolve_root(&dir, ws.config()).unwrap_or_else(|| dir.clone());
                            finish_load(st, load_project(ws, &root, &dir), &dir);
                        }
                    }
                }
            }
            TopMenu::File => {
                if menu_row(ui, "All files", st.view.file_filter.is_none()) {
                    st.view.file_filter = None;
                    st.modal = None;
                }
                for (i, f) in ws.files().iter().enumerate() {
                    if menu_row(ui, &f.spec.name, st.view.file_filter == Some(i)) {
                        st.view.file_filter = Some(i);
                        st.modal = None;
                    }
                }
            }
        }
    });
    if res.clicked_outside {
        st.modal = None;
    }
}

/// Menu des presets : s'ouvre vers le HAUT depuis le dropdown de la barre
/// batch (le dropdown doit rester visible).
pub(crate) fn draw_preset_menu(
    ctx: &egui::Context,
    anchor: Option<Rect>,
    ws: &mut Workspace,
    st: &mut EditorState,
) {
    if !st.presets.open {
        return;
    }
    let Some(anchor) = anchor else {
        return;
    };
    // The bottom of the menu must stay 8 px ABOVE the dropdown (which
    // must stay visible). Height: the one measured at the previous
    // frame, otherwise an always-overestimating estimate (the menu
    // can be too tall, never too short).
    let n = st.presets.list.len();
    let est = 32.0 * (1.0 + n as f32) + 130.0;
    let h = if st.presets.menu_h > 0.0 {
        st.presets.menu_h
    } else {
        est
    };
    let res = anchored_popup(ctx, "preset_menu", anchor, PopupSide::Above, h, |ui| {
        // ⚠️ `ui.cursor()` is in ABSOLUTE (screen) coordinates:
        // the menu height is measured on the rect of the Frame,
        // not on the cursor of the child ui (that's the role of the mold).
        ui.set_width(anchor.width());
        if menu_row(ui, "Vanilla", st.presets.choice == 0) {
            apply_preset_choice(ws, st, 0);
        }
        // A preset deleted during the iteration: we stop the
        // loop (the list is reloaded, the menu redraws itself).
        let mut deleted = false;
        for i in 0..n {
            let name = st.presets.list[i].name.clone();
            let active = st.presets.choice == i + 1;
            // A single allocation for the whole line; the body
            // (click = apply) and the trash bin (click = delete)
            // are sub-rects interacted with separately. Two
            // successive allocations would stack the trash bin
            // UNDER the line (top-down layout).
            let (row_rect, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::hover());
            let tr = Rect::from_min_max(
                pos2(row_rect.right() - 28.0, row_rect.top()),
                row_rect.right_bottom(),
            );
            let rr = Rect::from_min_max(
                row_rect.left_top(),
                pos2(row_rect.right() - 28.0, row_rect.bottom()),
            );
            let rr_resp = ui.interact(rr, ui.make_persistent_id(("preset_row", i)), Sense::click());
            let tr_resp = ui.interact(tr, ui.make_persistent_id(("preset_del", i)), Sense::click());
            let p = ui.painter();
            if active {
                p.rect_filled(rr, 0.0, c::BORDER);
            } else if rr_resp.hovered() {
                p.rect_filled(rr, 0.0, c::HOVER);
            }
            p.text(
                rr.left_center() + vec2(12.0, 0.0),
                Align2::LEFT_CENTER,
                &name,
                FontId::proportional(FONT_BODY),
                if active { c::TEXT_STRONG } else { c::TEXT_DIM },
            );
            if active {
                p.text(
                    rr.right_center() - vec2(12.0, 0.0),
                    Align2::RIGHT_CENTER,
                    "active",
                    FontId::proportional(FONT_SMALL),
                    c::TEXT_MUTED,
                );
            }
            // Corbeille : hover = fond + rouge ; clic = suppression.
            if tr_resp.hovered() {
                p.rect_filled(tr, 0.0, c::HOVER);
                draw_trash(p, tr.center(), c::DANGER_FG);
            } else {
                draw_trash(p, tr.center(), c::TEXT_MUTED);
            }
            if rr_resp.clicked() {
                apply_preset_choice(ws, st, i + 1);
            }
            if tr_resp.clicked() {
                match presets::delete(&ws.root, ws.config(), &name) {
                    Ok(()) => {
                        if active {
                            st.presets.choice = 0;
                        }
                        st.presets.list = presets::load_all(&ws.root, ws.config()).0;
                        st.presets.menu_h = 0.0; // re-estimer
                        st.msg = Some((true, format!("Preset \"{name}\" deleted.")));
                    }
                    Err(e) => st.msg = Some((false, e.to_string())),
                }
                deleted = true;
                break;
            }
        }
        if deleted {
            return;
        }
        hline(ui);
        ui.vertical_centered(|ui| {
            ui.add_space(8.0);
            let mut name = std::mem::take(&mut st.presets.new_name);
            let name_resp = ui.add(
                egui::TextEdit::singleline(&mut name)
                    .hint_text("New Preset…")
                    .desired_width(158.0),
            );
            st.presets.new_name = name;
            ui.add_space(8.0);
            let trimmed: String = st.presets.new_name.trim().to_string();
            // Doublon sur le slug (deux noms peuvent converger).
            let dup = st
                .presets
                .list
                .iter()
                .any(|p| presets::slug(&p.name) == presets::slug(&trimmed));
            // Enter (in the name field) = create, like the "+".
            // ⚠️ `has_focus()` is already FALSE on the frame of
            // the Enter (the TextEdit loses it in its own layout)
            // : it is `lost_focus()` + key pressed
            // on the same frame that characterizes the validation.
            let enter =
                name_resp.lost_focus() && name_resp.ctx.input(|i| i.key_pressed(egui::Key::Enter));
            if (primary_button(ui, "+", !trimmed.is_empty() && !dup).clicked() || enter)
                && !trimmed.is_empty()
                && !dup
            {
                match presets::save(ws, &trimmed) {
                    Ok(_) => {
                        st.presets.new_name.clear();
                        st.presets.list = presets::load_all(&ws.root, ws.config()).0;
                        st.presets.menu_h = 0.0; // re-estimer
                        st.msg = Some((true, format!("Preset \"{trimmed}\" saved.")));
                    }
                    Err(e) => st.msg = Some((false, e.to_string())),
                }
            }
        });
        ui.add_space(8.0);
    });
    st.presets.menu_h = res.meas_h;
    if res.clicked_outside {
        st.presets.open = false;
    }
}
