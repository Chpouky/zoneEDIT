//! Modal windows (one at a time): diff, confirmation, batch
//! (driven by the `[[batch]]` config), redirect.

use std::collections::{BTreeMap, BTreeSet};

use egui::RichText;

use crate::domain::{value, EntryId, EntryKind, FieldId};
use crate::io::resolve_root;
use crate::ui::icons::icon_for;
use crate::ui::metrics::{FONT_BODY, FONT_SMALL};
use crate::ui::ops::{
    apply_action, apply_preset, do_reload, finish_load, finish_reload, load_project,
};
use crate::ui::state::{ConfirmAction, EditorState, Modal};
use crate::ui::widgets::{anchored_popup, menu_note, PopupSide};
use crate::ui_kit::{c, hline, primary_button};

/// Width of the text laid out with the given style (same rendering as `ui.label`).
fn text_w(ui: &egui::Ui, text: &str, style: &egui::TextStyle) -> f32 {
    let font_id = style.resolve(ui.style());
    ui.fonts(|f| {
        f.layout(
            text.to_string(),
            font_id,
            egui::Color32::WHITE,
            f32::INFINITY,
        )
        .size()
        .x
    })
}

/// Largeur d'un `ui.button(label)` classique (texte + padding du style).
fn button_w(ui: &egui::Ui, label: &str) -> f32 {
    text_w(ui, label, &egui::TextStyle::Button) + 2.0 * ui.spacing().button_padding.x
}

/// Width of the primary button (`primary_button`, minimum width 44 px).
fn primary_button_w(ui: &egui::Ui, label: &str) -> f32 {
    button_w(ui, label).max(44.0)
}

/// Total width of a fields row (labels + 52 px inputs + spacing).
fn fields_row_width(ui: &egui::Ui, inputs: &[String]) -> f32 {
    const FIELD_W: f32 = 52.0;
    let mut total = 0.0;
    for (i, label) in inputs.iter().enumerate() {
        if i > 0 {
            total += 16.0;
        }
        total += text_w(ui, &format!("{label} :"), &egui::TextStyle::Body) + 4.0 + FIELD_W;
    }
    total
}

/// Height of the icon of a diff item row (smaller than the main table:
/// the popup must stay compact).
const DIFF_ICON_H: f32 = 20.0;

/// Value of a modified field: "label   orig → current".
fn draw_field_value(ui: &mut egui::Ui, ws: &crate::domain::Workspace, fid: FieldId) {
    let f = ws.field(fid);
    let spec = ws.spec(f.kind);
    // Spawn chances: displayed in percent.
    let (orig, cur) = if spec.is_percent() {
        (
            value::display(spec, f.original()),
            value::display(spec, f.value()),
        )
    } else {
        (f.original().to_string(), f.value().to_string())
    };
    ui.colored_label(c::TEXT_MUTED, spec.short.clone());
    ui.add_space(4.0);
    ui.colored_label(c::TEXT_MUTED, orig);
    ui.add_space(2.0);
    ui.colored_label(c::TEXT_DIM, "→");
    ui.add_space(2.0);
    ui.colored_label(c::MODIFIED_FG, cur);
}

/// Draws the open modal (if `st.modal` is `Some`), then applies the
/// actions accumulated during the frame (batched, after the drawing).
pub(crate) fn draw_modals(
    ctx: &egui::Context,
    ws: &mut crate::domain::Workspace,
    st: &mut EditorState,
) {
    // ---- Window: diff ------------------------------------------------------
    if matches!(st.modal, Some(Modal::Diff)) {
        let mut diff_open = true;
        egui::Window::new("Diff preview")
            .open(&mut diff_open)
            .resizable(true)
            .collapsible(false)
            .min_width(440.0)
            .show(ctx, |ui| {
                ui.set_min_height(260.0);
                let ids: Vec<_> = ws.dirty_fields().collect();
                if ids.is_empty() {
                    ui.label(RichText::new("No pending changes.").color(c::TEXT_MUTED));
                } else {
                    // Compact top-to-bottom structure:
                    //   File → block (dropdown) → item (icon + name),
                    // all the modified fields of an item on the same line
                    // (Min and Max changed = one line, not two).
                    let mut by_entry: BTreeMap<EntryId, Vec<FieldId>> = BTreeMap::new();
                    let mut orphan: Vec<FieldId> = Vec::new();
                    for id in &ids {
                        match ws.owner(*id) {
                            Some(e) => by_entry.entry(e).or_default().push(*id),
                            None => orphan.push(*id),
                        }
                    }
                    // Display order: file, then block (line of the
                    // struct.begin), then entry (arena order = file).
                    let mut entries: Vec<EntryId> = by_entry.keys().copied().collect();
                    entries.sort_by_key(|&eid| {
                        let e = ws.entry(eid);
                        (e.file, ws.block(e.block).line, eid.idx())
                    });
                    orphan.sort_by_key(|&id| {
                        let f = ws.field(id);
                        (f.file, f.line)
                    });

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let (mut cur_file, mut cur_block) = (usize::MAX, usize::MAX);
                        let file_header = |ui: &mut egui::Ui, file: usize| {
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new(ws.files()[file].name().to_string())
                                    .monospace()
                                    .size(12.0)
                                    .strong()
                                    .color(c::TEXT_DIM),
                            );
                        };
                        for &eid in &entries {
                            let e = ws.entry(eid);
                            if e.file != cur_file {
                                cur_file = e.file;
                                cur_block = usize::MAX;
                                file_header(ui, e.file);
                            }
                            if e.block.idx() != cur_block {
                                cur_block = e.block.idx();
                                ui.horizontal(|ui| {
                                    ui.add_space(14.0);
                                    ui.label(
                                        RichText::new(ws.block(e.block).name.clone())
                                            .monospace()
                                            .size(11.0)
                                            .color(c::TEXT_MUTED),
                                    );
                                });
                            }
                            // Item row: icon + name (+ context), then the
                            // modified values, on the same line.
                            let fields = by_entry.get(&eid).unwrap();
                            let icon = icon_for(e.icon_key());
                            let scale =
                                (DIFF_ICON_H / icon.size.y).min((2.0 * DIFF_ICON_H) / icon.size.x);
                            ui.horizontal(|ui| {
                                ui.add_space(28.0);
                                ui.add(
                                    egui::Image::new(icon.src.clone())
                                        .fit_to_exact_size(icon.size * scale),
                                );
                                ui.add_space(6.0);
                                ui.label(RichText::new(e.display_name()).size(FONT_BODY));
                                // Context (rank · caliber · section): useful
                                // when the same SID appears several times.
                                let lbl = e.ctx.label();
                                if !lbl.is_empty() {
                                    ui.label(
                                        RichText::new(lbl).size(FONT_SMALL).color(c::TEXT_MUTED),
                                    );
                                }
                                ui.add_space(14.0);
                                for (i, fid) in fields.iter().enumerate() {
                                    if i > 0 {
                                        ui.add_space(16.0);
                                    }
                                    draw_field_value(ui, ws, *fid);
                                }
                            });
                        }
                        // Fields without a carrying entry (defensive): line per
                        // file, with the line number instead of the item.
                        for fid in &orphan {
                            let f = ws.field(*fid);
                            if f.file != cur_file {
                                cur_file = f.file;
                                file_header(ui, f.file);
                            }
                            ui.horizontal(|ui| {
                                ui.add_space(28.0);
                                ui.label(
                                    RichText::new(format!("L{}", f.line))
                                        .monospace()
                                        .size(11.0)
                                        .color(c::TEXT_MUTED),
                                );
                                ui.add_space(10.0);
                                draw_field_value(ui, ws, *fid);
                            });
                        }
                        ui.add_space(6.0);
                    });
                }
            });
        if !diff_open {
            st.modal = None;
        }
    }

    // ---- Window: confirmation ---------------------------------------------
    if let Some(Modal::Confirm(action)) = st.modal.clone() {
        let (title, verb) = match &action {
            ConfirmAction::Reload => ("Reload files?", "Reload"),
            ConfirmAction::LoadProject(_) => ("Switch project?", "Switch"),
            ConfirmAction::OpenFolder(_) => ("Open folder?", "Open"),
            ConfirmAction::ApplyPreset(_) => ("Apply preset?", "Apply"),
        };
        let n = ws.dirty_count();
        egui::Window::new(title)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label(format!(
                    "You have {n} pending change(s). They will be lost."
                ));
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        st.modal = None;
                    }
                    if primary_button(ui, verb, true).clicked() {
                        match &action {
                            ConfirmAction::Reload => finish_reload(st, do_reload(ws)),
                            ConfirmAction::LoadProject(p) => {
                                finish_load(st, load_project(ws, p, p), p);
                            }
                            // `p` is the CHOSEN folder: we re-resolve the
                            // root at the time of the confirmation.
                            ConfirmAction::OpenFolder(p) => {
                                let root = resolve_root(p, ws.config()).unwrap_or(p.clone());
                                finish_load(st, load_project(ws, &root, p), p);
                            }
                            ConfirmAction::ApplyPreset(i) => {
                                if *i == 0 {
                                    ws.reset_all();
                                } else if let Some(p) = st.presets.list.get(i - 1) {
                                    let (ok, msg) = apply_preset(ws, p);
                                    st.msg = Some((ok, msg));
                                }
                                st.presets.choice = *i;
                            }
                        }
                        st.modal = None;
                    }
                });
            });
    }

    // ---- Batch popups ---------------------------------------------------------
    // Buttons and fields driven by the configuration (`[[batch]]`): a single
    // generic mold for all. Same mold as the other menus
    // (`anchored_popup`): anchored ABOVE the batch button (like the PRESET
    // dropdown), presets menu design (note line + separator + centered
    // section). The outside click closes the popup, Escape too. The popup
    // height is measured at the previous frame (`meas_h`).
    if let Some(Modal::Batch {
        idx,
        mut inputs,
        anchor,
        meas_h,
    }) = st.modal.clone()
    {
        let mut closed = false;
        let h = if meas_h > 0.0 { meas_h } else { 110.0 };
        let spec = ws.config().batch[idx].clone();
        let res = anchored_popup(ctx, "modal_batch", anchor, PopupSide::Above, h, |ui| {
            // Measurement of the content widths (fields + buttons) to:
            //  1) size the popup as tightly as possible (no fixed width),
            //  2) manually center each line. In this version of egui,
            //     `horizontal_centered` only centers VERTICALLY: the real
            //     horizontal centering goes through a computed head space.
            let fw = fields_row_width(ui, &spec.inputs);
            let bw = button_w(ui, "Cancel") + 8.0 + primary_button_w(ui, "Apply");
            // 12 px of padding * 2 + ~12 px of centering margin * 2.
            let w = fw.max(bw) + 48.0;
            ui.set_width(w);
            menu_note(ui, &format!("{} selected row(s)", st.selection.batch.len()));
            hline(ui);
            ui.add_space(8.0);
            // Fields row: centered grouping, narrow fields
            // (small values, max 2 characters).
            let flead = (w - 24.0 - fw).max(0.0) / 2.0;
            ui.horizontal(|ui| {
                ui.add_space(12.0 + flead);
                for (i, input) in inputs.iter_mut().enumerate() {
                    if i > 0 {
                        ui.add_space(16.0);
                    }
                    let label = spec.inputs.get(i).map(|s| s.as_str()).unwrap_or("");
                    ui.label(format!("{label} :"));
                    ui.add_space(4.0);
                    ui.add(egui::TextEdit::singleline(input).desired_width(52.0));
                }
            });
            ui.add_space(8.0);
            // Buttons: centered grouping.
            let bleed = (w - 24.0 - bw).max(0.0) / 2.0;
            ui.horizontal(|ui| {
                ui.add_space(12.0 + bleed);
                if ui.button("Cancel").clicked() {
                    closed = true;
                }
                ui.add_space(8.0);
                if primary_button(ui, "Apply", true).clicked() {
                    // (field, value) pairs: empty fields are
                    // skipped (no application).
                    let mut pairs: Vec<(crate::domain::FieldKind, &str)> = Vec::new();
                    for (i, field) in spec.fields.iter().enumerate() {
                        let t = inputs.get(i).map(|s| s.trim()).unwrap_or("");
                        if !t.is_empty() {
                            pairs.push((*field, t));
                        }
                    }
                    if pairs.is_empty() {
                        st.msg = Some((false, "Nothing to apply.".into()));
                    } else {
                        let entries: Vec<_> = st
                            .selection
                            .batch
                            .iter()
                            .filter_map(|k| ws.owner(*k))
                            .collect::<BTreeSet<_>>()
                            .into_iter()
                            .collect();
                        let rep = ws.batch_input(&entries, &pairs);
                        st.msg = Some((
                            rep.rejected == 0,
                            format!(
                                "{} field(s) applied, {} rejected.",
                                rep.applied, rep.rejected
                            ),
                        ));
                    }
                    closed = true;
                }
            });
            ui.add_space(8.0);
        });
        if closed {
            st.modal = None;
        } else if !res.clicked_outside && !ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            st.modal = Some(Modal::Batch {
                idx,
                inputs,
                anchor,
                meas_h: res.meas_h,
            });
        }
    }

    // ---- Window: redirect (orphan) -----------------------------------------
    if let Some(Modal::Redirect(target)) = st.modal.clone() {
        egui::Window::new(format!("Redirect — {target}"))
            .resizable(true)
            .min_width(420.0)
            .show(ctx, |ui| {
                ui.colored_label(
                    c::TEXT_DIM,
                    "This target has no matching block in the loaded files.",
                );
                ui.label(
                    RichText::new(
                        "Re-targeting is manual : edit the `→ <target>` line in the .cfg source.",
                    )
                    .size(12.0)
                    .color(c::TEXT_MUTED),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Existing blocks you could point it to :")
                        .size(12.0)
                        .color(c::TEXT_DIM),
                );
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        // Distinct blocks, in the order of the entries.
                        let mut seen = std::collections::HashSet::new();
                        for id in ws.entry_ids() {
                            let e = ws.entry(id);
                            if e.kind != EntryKind::Item {
                                continue;
                            }
                            if seen.insert(e.block) {
                                ui.label(
                                    RichText::new(ws.block(e.block).name.clone())
                                        .monospace()
                                        .size(12.0),
                                );
                            }
                        }
                    });
            });
    }

    // Actions accumulated during the frame: applied in batch after the drawing.
    for a in st.take_actions() {
        apply_action(ws, st, a);
    }
}
