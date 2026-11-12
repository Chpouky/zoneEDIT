//! zoneEDIT UI — "A – Dark" redesign matching the references
//! (`zoneEDIT_references/`: spec + preview + mockup).

pub(crate) mod icons;
pub(crate) mod layout;
pub(crate) mod menus;
pub(crate) mod metrics;
pub(crate) mod modals;
pub(crate) mod ops;
pub(crate) mod rows;
pub(crate) mod sidebar;
pub(crate) mod state;
pub(crate) mod statusbar;
pub(crate) mod table;
#[cfg(test)]
pub(crate) mod tests;
pub(crate) mod widgets;

pub(crate) use metrics::*;
pub(crate) use state::*;
pub(crate) use table::*;
pub(crate) use widgets::*;

use std::collections::HashMap;

use egui::{pos2, vec2, Align, Align2, Color32, FontId, Layout, Rect, RichText, Sense, Stroke};

use crate::config::Config;
use crate::domain::{BlockId, EntryId, Workspace};
use crate::io;

#[cfg(test)]
use crate::domain::Entry;
use crate::ui_kit::{c, chip, primary_button};

pub struct ZoneEditApp {
    ws: Option<Workspace>,
    st: EditorState,
}

impl ZoneEditApp {
    /// No folder is auto-loaded at startup, unless a previous session
    /// remembered one: the persisted last folder is opened silently when
    /// it still loads. Otherwise the app shows the top bar and a hint,
    /// and the user picks the root from the "Folder" menu.
    pub fn new() -> Self {
        let mut app = Self {
            ws: None,
            st: EditorState::default(),
        };
        if let Some(dir) = io::appstate::load().last_root {
            let root = io::load::resolve_root(&dir, &Config::default_arc()).unwrap_or(dir.clone());
            if let Ok(mut w) = io::load::load(&root) {
                w.chosen_dir = dir;
                app.st.presets.list = io::presets::load_all(&w.root, w.config()).0;
                app.ws = Some(w);
            }
            // Folder missing or unreadable: start empty, the user re-picks.
        }
        app
    }
}

impl Default for ZoneEditApp {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for ZoneEditApp {
    /// eframe's default clear color is semi-transparent:
    /// we force an opaque background, otherwise the banner is transparent.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        c::BG.to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // The image loader is installed only once in `main.rs`
        // (plan 4.4).
        let Self { ws, st } = self;
        // Frame time (plan 4.1): displayed in the status bar
        // (previous frame's value — measured here, read at draw time).
        let t0 = std::time::Instant::now();
        if let Some(ws) = ws {
            show_editor(ctx, ws, st);
        } else {
            // No root at startup: the app shows its interface empty
            // (top bar + hint); the user opens a folder with the existing
            // "Folder" menu → "Open folder…". No dialog at startup.
            show_no_folder(ctx, ws, st);
        }
        st.frame_ms = t0.elapsed().as_secs_f32() * 1000.0;
    }
}
// ---------------------------------------------------------------------------
// No folder loaded (empty state)
// ---------------------------------------------------------------------------

/// Interface shell shown before any folder is opened: the usual top bar
/// (with the "Folder" dropdown) and a centered hint in the empty body.
/// The "Folder" menu only offers the existing "Open folder…" action, since
/// there is no workspace yet to list projects from.
pub(crate) fn show_no_folder(
    ctx: &egui::Context,
    ws: &mut Option<Workspace>,
    st: &mut EditorState,
) {
    // Esc closes the menu.
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        st.modal = None;
    }

    let mut menu_anchor: Option<Rect> = None;

    // ---- Top bar (52) ------------------------------------------------------
    egui::TopBottomPanel::top("topbar")
        .exact_height(TOPBAR_H)
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            ui.painter().rect_filled(rect, 0.0, c::PANEL);
            ui.horizontal_centered(|ui| {
                ui.add_space(12.0);
                ui.label(
                    RichText::new("zoneEDIT")
                        .size(15.0)
                        .strong()
                        .color(c::ACCENT),
                );
                ui.add_space(14.0);
                // Vertical separator, centered on the bar (not on the line).
                ui.painter().rect_filled(
                    Rect::from_min_size(
                        pos2(ui.cursor().min.x, rect.center().y - 12.0),
                        vec2(1.0, 24.0),
                    ),
                    0.0,
                    c::BORDER,
                );
                ui.add_space(14.0);
                dim12(ui, "Folder");
                ui.add_space(6.0);
                let r = dropdown(ui, "Select a folder…", 170.0);
                if r.clicked() {
                    st.modal = match st.modal.as_ref() {
                        Some(Modal::Menu(TopMenu::Project)) => None,
                        _ => Some(Modal::Menu(TopMenu::Project)),
                    };
                }
                if matches!(st.modal, Some(Modal::Menu(TopMenu::Project))) {
                    menu_anchor = Some(r.rect);
                }
                ui.add_space(12.0);
            });
        });

    // ---- "Folder" menu: only "Open folder…" (no workspace to list) --------
    if let Some(anchor) = menu_anchor {
        let res = anchored_popup(ctx, "top_menu", anchor, PopupSide::Below, 0.0, |ui| {
            ui.set_width(anchor.width().max(220.0));
            if menu_row(ui, "Open folder…", false) {
                // Native OS folder picker (rfd), same action as the
                // loaded editor's "Folder" menu.
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    let cfg = Config::default_arc();
                    let root = io::load::resolve_root(&dir, &cfg).unwrap_or(dir.clone());
                    match io::load::load(&root) {
                        Ok(mut w) => {
                            // The "Folder" dropdown will show the folder
                            // chosen in the explorer (not the resolved root).
                            let name = dir
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default();
                            let presets = io::presets::load_all(&w.root, w.config()).0;
                            io::appstate::save_last_root(&dir);
                            w.chosen_dir = dir;
                            st.reset_view();
                            st.presets.list = presets;
                            st.msg = Some((true, format!("Loaded project {name}.")));
                            *ws = Some(w);
                        }
                        Err(e) => {
                            rfd::MessageDialog::new()
                                .set_title("zoneEDIT")
                                .set_level(rfd::MessageLevel::Error)
                                .set_description(format!("Cannot load {} :\n{e}", root.display()))
                                .show();
                            st.modal = None;
                        }
                    }
                } else {
                    // Picker canceled: close the menu.
                    st.modal = None;
                }
            }
        });
        if res.clicked_outside {
            st.modal = None;
        }
    }

    // ---- Central panel: hint -------------------------------------------------
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(c::BG))
        .show(ctx, |ui| {
            ui.vertical_centered_justified(|ui| {
                ui.add_space(90.0);
                ui.label(
                    RichText::new("No folder loaded")
                        .size(20.0)
                        .strong()
                        .color(c::TEXT_STRONG),
                );
                ui.add_space(10.0);
                ui.label(
                    RichText::new("Choose your STALKER2ZoneKit folder with the \"Folder\" menu.")
                        .size(14.0)
                        .color(c::TEXT_DIM),
                );
            });
        });
}

// ---------------------------------------------------------------------------
// Editor
// ---------------------------------------------------------------------------

pub(crate) fn show_editor(ctx: &egui::Context, ws: &mut Workspace, st: &mut EditorState) {
    // Esc closes menus / modals / editing.
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        st.modal = None;
        st.presets.open = false;
        st.section_menu = false;
        st.editing.cell = None;
    }

    // Ephemeral message: clears itself after 4 s (displayed in the
    // status bar, no dedicated bar anymore).
    if st.msg.is_some() {
        let t = ctx.input(|i| i.time);
        let since = *st.msg_since.get_or_insert(t);
        if t - since >= 4.0 {
            st.msg = None;
            st.msg_since = None;
        }
    }

    // Derived state of the frame: computed only once here (step 3.8);
    // the drawing code below no longer re-scans the workspace.
    let derived = compute_derived(ws, st);

    // Files modified on disk during the session → reset banner
    // (confirmation if there are unexported modifications).
    let stale = io::load::stale_files(ws);
    if !stale.is_empty() {
        egui::TopBottomPanel::top("stale_files")
            .frame(bar_frame())
            .show_separator_line(false) // bar_frame already has its stroke
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(
                        c::DANGER_FG,
                        format!("File modified externally : {}", stale.join(", ")),
                    );
                    if ui.button("Reload").clicked() {
                        st.push_action(Action::Reload);
                    }
                });
            });
    }

    let mut menu_anchor: Option<(TopMenu, Rect)> = None;
    let mut preset_anchor: Option<Rect> = None;

    // ---- Top bar (52) ------------------------------------------------------
    egui::TopBottomPanel::top("topbar")
        .exact_height(TOPBAR_H)
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            ui.painter().rect_filled(rect, 0.0, c::PANEL);
            // Bottom border: drawn by the egui fork (show_separator_line),
            // we don't draw a second one.
            ui.horizontal_centered(|ui| {
                ui.add_space(12.0);
                ui.label(
                    RichText::new("zoneEDIT")
                        .size(15.0)
                        .strong()
                        .color(c::ACCENT),
                );
                ui.add_space(14.0);
                // Vertical separator, centered on the bar (not on the line).
                ui.painter().rect_filled(
                    Rect::from_min_size(
                        pos2(ui.cursor().min.x, rect.center().y - 12.0),
                        vec2(1.0, 24.0),
                    ),
                    0.0,
                    c::BORDER,
                );
                ui.add_space(14.0);

                dim12(ui, "Folder");
                ui.add_space(6.0);
                // Name of the CHOOSEN folder in the explorer (e.g. the ZoneKit,
                // not the resolved root "GameLite"), with fallback to the
                // root if the name is not available.
                let folder_name = ws
                    .chosen_dir
                    .file_name()
                    .and_then(|s| s.to_str())
                    .filter(|s| !s.is_empty())
                    .or_else(|| ws.root.file_name().and_then(|s| s.to_str()))
                    .unwrap_or("?")
                    .to_string();
                let r = dropdown(ui, &folder_name, 170.0);
                if r.clicked() {
                    st.section_menu = false;
                    st.modal = match st.modal.as_ref() {
                        Some(Modal::Menu(TopMenu::Project)) => None,
                        _ => {
                            // The project list is recomputed on menu OPENING,
                            // not on every frame (plan 4.3).
                            st.project_cache = None;
                            Some(Modal::Menu(TopMenu::Project))
                        }
                    };
                }
                if matches!(st.modal, Some(Modal::Menu(TopMenu::Project))) {
                    preset_anchor = None;
                    st.presets.open = false;
                    menu_anchor = Some((TopMenu::Project, r.rect));
                }

                ui.add_space(16.0);
                dim12(ui, "File");
                ui.add_space(6.0);
                let file_label = match st.view.file_filter {
                    None => "All files".to_string(),
                    Some(i) => ws
                        .files()
                        .get(i)
                        .map(|f| f.spec.name.clone())
                        .unwrap_or_else(|| "All files".into()),
                };
                let r = dropdown(ui, &file_label, 250.0);
                if r.clicked() {
                    st.section_menu = false;
                    st.modal = match st.modal.as_ref() {
                        Some(Modal::Menu(TopMenu::File)) => None,
                        _ => Some(Modal::Menu(TopMenu::File)),
                    };
                }
                if matches!(st.modal, Some(Modal::Menu(TopMenu::File))) {
                    preset_anchor = None;
                    st.presets.open = false;
                    menu_anchor = Some((TopMenu::File, r.rect));
                }

                ui.add_space(16.0);
                if btn(ui, "Reload").clicked() {
                    st.push_action(Action::Reload);
                }

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if primary_button(
                        ui,
                        &format!("Export ({})", derived.dirty),
                        derived.dirty > 0 && derived.n_err == 0,
                    )
                    .clicked()
                    {
                        st.push_action(Action::Export);
                    }
                    ui.add_space(8.0);
                    if btn(ui, "Diff preview").clicked() {
                        st.push_action(Action::ShowDiff);
                    }
                });
                ui.add_space(12.0);
            });
        });

    // ---- Sidebar (250) -----------------------------------------------------
    sidebar::draw_sidebar(ctx, ws, st, &derived);

    // ---- Status bar (32) + errors panel ------------------------
    // (before the central panel: a bottom panel shown after does not shrink
    //  the space already allocated to the central panel)
    statusbar::draw_statusbar(ctx, st, &derived);
    statusbar::draw_errors_panel(ctx, ws, st);

    // ---- Central panel ---------------------------------------------------
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(c::BG))
        .show(ctx, |ui| {
            let w = ui.available_width();
            // No vertical spacing between filters / body / batch bar:
            // egui's made the batch overflow under the status bar.
            ui.spacing_mut().item_spacing.y = 0.0;

            // Filter bar (44)
            let (fb, _) = ui.allocate_exact_size(vec2(w, FILTER_H), Sense::hover());
            ui.painter().rect_filled(fb, 0.0, c::PANEL);
            ui.painter().line_segment(
                [fb.left_bottom(), fb.right_bottom()],
                Stroke::new(1.0_f32, c::BORDER),
            );
            let mut filter_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(fb)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            filter_ui.add_space(12.0);
            {
                let ui = &mut filter_ui;
                dim12(ui, "Filters");
                ui.add_space(8.0);
                // One chip per rank of the configuration (bit `i` = rank `i`)
                // + "No Rank": cumulative filters (union), no chip
                // selected = show everything.
                for (i, rank) in ws.config().ranks.iter().enumerate() {
                    let bit = 1u32 << i;
                    if chip(ui, rank, st.view.difficulty & bit != 0).clicked() {
                        st.view.difficulty ^= bit;
                    }
                }
                // "No Rank": entries without rank (neither a Rank field, nor a rank
                // in the block name).
                if chip(ui, "No Rank", st.view.no_rank).clicked() {
                    st.view.no_rank = !st.view.no_rank;
                }
                // "Trade": entries containing "trade" (SID, block or
                // context), case-insensitive.
                if chip(ui, "Trade", st.view.trade).clicked() {
                    st.view.trade = !st.view.trade;
                }
                ui.add_space(14.0);
                // Vertical separator, centered on the bar (not on the line).
                ui.painter().rect_filled(
                    Rect::from_min_size(
                        pos2(ui.cursor().min.x, fb.center().y - 10.0),
                        vec2(1.0, 20.0),
                    ),
                    0.0,
                    c::BORDER,
                );
                ui.add_space(14.0);
                dim12(ui, "Show refs");
                ui.add_space(8.0);
                // Checkbox: clickable area 18×18, 13 px box centered.
                let (cb_rect, cb) = ui.allocate_exact_size(vec2(CB_HIT, CB_HIT), Sense::click());
                if cb.clicked() {
                    st.view.show_refs = !st.view.show_refs;
                }
                draw_checkbox(
                    ui.painter(),
                    cb_rect.center(),
                    if st.view.show_refs {
                        CbState::Checked
                    } else {
                        CbState::Off
                    },
                );
                // Search field, right-aligned (moved here from the sidebar).
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(12.0);
                    draw_search_field(ui, st, SEARCH_W);
                });
            }

            // Body: row scroll (vertical + horizontal) + sticky header + batch bar
            let batch_h = BATCH_H;
            let body_h = (ui.available_height() - batch_h).max(0.0);
            let cols = columns_for(ws, st.view.category);
            // Header labels: the `long` names (the column headers of the
            // config). The width of each value column is measured on them —
            // the label is never truncated; when the name zone or the
            // columns do not fit in the window, the table scrolls
            // horizontally. The `short` names live in the field modal.
            let hdr_font = FontId::proportional(FONT_SMALL);
            let labels: Vec<String> = cols
                .iter()
                .map(|col| {
                    let spec = ws.spec(*col);
                    if spec.is_percent() {
                        // Spawn chances / bleed chances are shown in %.
                        format!("{} %", spec.long.to_uppercase())
                    } else {
                        spec.long.to_uppercase()
                    }
                })
                .collect();
            // Column geometry in body-local coordinates (0 = left edge of
            // the body); the rows add their own left edge (which carries
            // the horizontal scroll offset) when drawing the cells.
            let geo = col_layout(ui.painter(), w, &labels, &hdr_font);
            let body = Rect::from_min_size(ui.cursor().min, vec2(w, body_h));
            let mut body_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(body)
                    .layout(Layout::top_down(Align::LEFT)),
            );
            {
                let ui = &mut body_ui;
                // Sticky header: the first 30 px of the body, always
                // stuck against the filter bar. The scroll is placed
                // BELOW the header (and not above with an add_space).
                let hdr = Rect::from_min_size(ui.cursor().min, vec2(w, HEADER_H));
                // The scroll is shown BEFORE the header: the ScrollArea's clip
                // extends `visuals.clip_rect_margin` (3px) beyond its rect, so
                // when scrolling the rows overflow 3px above the top of the scroll.
                // Painting the header (opaque fill) after masks this overflow
                // and the row of labels keeps a constant thickness.
                let scroll_rect = Rect::from_min_max(pos2(hdr.min.x, hdr.max.y), body.max);
                let mut scroll_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(scroll_rect)
                        .layout(Layout::top_down(Align::LEFT)),
                );
                // Offset the rows will be drawn at this frame: the ScrollArea
                // loads it from storage BEFORE processing this frame's wheel,
                // so the header must use this same (pre-update) value to
                // stay in lockstep with the rows while scrolling.
                // NOTE: `ScrollArea::id_salt(x)` stores `Id::new(x)`, so the
                // state id is `make_persistent_id(Id::new(Id::new("rows")))`
                // (the double wrap is intentional — it must match `begin()`).
                let rows_id = scroll_ui.make_persistent_id(egui::Id::new(egui::Id::new("rows")));
                let pre_off_x = egui::containers::scroll_area::State::load(ctx, rows_id)
                    .map(|s| s.offset.x)
                    .unwrap_or(0.0);
                egui::ScrollArea::both()
                    .id_salt(egui::Id::new("rows"))
                    // Keep the full viewport size even when the rows are
                    // short: the horizontal scrollbar stays pinned to the
                    // bottom of the row window (not under the last row).
                    .auto_shrink(false)
                    .show(&mut scroll_ui, |ui| {
                        render_table(ui, ws, st, &derived, &cols, &geo);
                    });
                ui.painter().rect_filled(hdr, 0.0, c::INPUT_BG);
                ui.painter().line_segment(
                    [hdr.left_bottom(), hdr.right_bottom()],
                    Stroke::new(1.0_f32, c::BORDER),
                );
                // Sticky header: the labels follow the horizontal scroll
                // offset (the pre-update one, see `pre_off_x`) and are
                // clipped to the header band (the child ui clips its
                // painter to its own rect).
                let off = -pre_off_x;
                let hdr_ui = ui.new_child(egui::UiBuilder::new().max_rect(hdr));
                let p = hdr_ui.painter();
                p.text(
                    hdr.left_center() + vec2(GEN_TRI_X + off, 0.0),
                    Align2::LEFT_CENTER,
                    "GENERATOR / ITEM",
                    hdr_font.clone(),
                    c::TEXT_MUTED,
                );
                for (i, _) in cols.iter().enumerate() {
                    p.text(
                        pos2(hdr.left() + off + geo.cols[i].center, hdr.center().y),
                        Align2::CENTER_CENTER,
                        labels[i].clone(),
                        hdr_font.clone(),
                        c::TEXT_MUTED,
                    );
                }
            }
            ui.advance_cursor_after_rect(body);

            // Batch bar (52) — buttons driven by the configuration
            // (`[[batch]]`): the label comes from the TOML, no more hardcoded texts.
            let (bb, _) = ui.allocate_exact_size(vec2(w, batch_h), Sense::hover());
            ui.painter().rect_filled(bb, 0.0, c::PANEL);
            ui.painter().line_segment(
                [bb.left_top(), bb.right_top()],
                Stroke::new(1.0_f32, c::BORDER),
            );
            let mut batch_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(bb)
                    .layout(Layout::left_to_right(Align::Center)),
            );
            batch_ui.add_space(12.0);
            {
                let ui = &mut batch_ui;
                dim12(ui, &format!("{} row(s) selected", st.selection.batch.len()));
                ui.add_space(16.0);
                // The batch buttons: the button rect is captured to
                // anchor the popup above (like the PRESET dropdown).
                for idx in 0..ws.config().batch.len() {
                    if idx > 0 {
                        ui.add_space(8.0);
                    }
                    let r = ui.add_enabled(
                        !st.selection.batch.is_empty(),
                        egui::Button::new(
                            RichText::new(ws.config().batch[idx].label.clone()).size(12.0),
                        )
                        .min_size(vec2(44.0, 32.0)),
                    );
                    if r.clicked() {
                        st.push_action(Action::Batch {
                            idx,
                            anchor: r.rect,
                        });
                    }
                }
                ui.add_space(8.0);
                if btn(ui, "Reset Selected").clicked() {
                    st.push_action(Action::ResetSelection);
                }
                ui.add_space(8.0);
                if btn(ui, "Reset All").clicked() {
                    st.push_action(Action::ResetAll);
                }
                // PRESET dropdown anchored to the right edge of the batch bar
                // (moved from the footer of the sidebar).
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(12.0);
                    let label = preset_label(st);
                    let r = dropdown(ui, &label, 200.0);
                    if r.clicked() {
                        st.presets.open = !st.presets.open;
                        st.modal = None;
                        st.section_menu = false;
                        menu_anchor = None;
                    }
                    if st.presets.open {
                        preset_anchor = Some(r.rect);
                    }
                    ui.add_space(8.0);
                    dim12(ui, "PRESET");
                });
            }
        });

    // ---- Menus (topbar / presets) ------------------------------------------
    // Closing on outside click is handled by each popup
    // (`PopupResult::clicked_outside` in the `anchored_popup` mold).
    if let Some((ref menu, anchor)) = menu_anchor {
        menus::draw_topbar_menu(ctx, menu, anchor, ws, st);
    }
    menus::draw_preset_menu(ctx, preset_anchor, ws, st);

    // ---- Modal windows (+ application of actions at end of frame) -----
    modals::draw_modals(ctx, ws, st);
}

/// Identifiers of the visible entries according to the filters (category +
/// search + file + difficulty). Same predicate as
/// `entries_of` + the filters, but without cloning `Entry` (step 3.8).
pub(crate) fn visible_indices(ws: &Workspace, st: &EditorState) -> Vec<EntryId> {
    let search = st.view.search.trim().to_lowercase();
    ws.entry_ids()
        .filter(|&id| {
            ws.in_category(id, st.view.category)
                && (search.is_empty() || ws.search_key(id).contains(&search))
                && st.view.file_filter.is_none_or(|f| f == ws.entry(id).file)
                && matches_difficulty(&ws.entry(id).ctx, st.view.difficulty, st.view.no_rank)
                && (!st.view.trade || ws.search_key(id).contains("trade"))
        })
        .collect()
}

/// Visible entries, as values (clones — headless tests usage only;
/// the application uses `visible_indices`, without cloning).
#[cfg(test)]
pub(crate) fn visible_entries(ws: &Workspace, st: &EditorState) -> Vec<Entry> {
    visible_indices(ws, st)
        .into_iter()
        .map(|id| ws.entry(id).clone())
        .collect()
}

/// Computes the derived state of the frame in one pass (step 3.8).
pub(crate) fn compute_derived(ws: &Workspace, st: &EditorState) -> Derived {
    let visible_idx = visible_indices(ws, st);
    let block_entries: HashMap<crate::domain::BlockId, Vec<EntryId>> = visible_idx
        .iter()
        .map(|&id| (ws.entry(id).block, id))
        .fold(HashMap::new(), |mut m, (k, id)| {
            m.entry(k).or_default().push(id);
            m
        });
    // Delegation highlighting: each visible delegation receives the
    // next tint from the palette (screen order); its row, its source block
    // and the target block share this tint. The first assignment
    // wins if a block is involved in several delegations.
    let mut delegation_tint: HashMap<EntryId, Color32> = HashMap::new();
    let mut block_tint: HashMap<BlockId, Color32> = HashMap::new();
    let mut n_deleg = 0;
    for &id in &visible_idx {
        let e = ws.entry(id);
        if !e.is_delegation() {
            continue;
        }
        let tint = c::GROUP_TINTS[n_deleg % c::GROUP_TINTS.len()];
        n_deleg += 1;
        delegation_tint.insert(id, tint);
        block_tint.entry(e.block).or_insert(tint);
        let target = ws
            .blocks()
            .iter()
            .position(|b| b.name == e.sid)
            .map(|i| BlockId(i as u32));
        if let Some(tb) = target {
            block_tint.entry(tb).or_insert(tint);
        }
    }

    // Block-entry hierarchy (anomalies, weapons). A block is a COLOR UNIT
    // when it has children AND sits at depth 0 or 1 in the `refkey` tree
    // (file root, or direct child of the root) — the "family" level. It
    // takes the next palette tint (screen order); each visible block of
    // the tree is then tinted with its CLOSEST color-unit ancestor (itself
    // when it is one). Deeper parents do NOT open a new color: the real
    // weapon GeneralSetup is 4 levels deep (TemplateWeapon → family
    // template → gun → gun variant, 29 guns are parents of variants), so
    // a per-parent tint would burn the palette — instead every pistol
    // variant keeps the pistol-family tint, every SMG the SMG tint, and
    // each of the 12 families stays distinct. The anomaly data (2 levels)
    // is unchanged: the bases are depth-0 units, the variants inherit.
    let (anom_parent, anom_children) = anomaly_hierarchy(ws);
    let depth_of = |b: BlockId| -> usize {
        let mut d = 0;
        let mut cur = b;
        while let Some(&p) = anom_parent.get(&cur) {
            d += 1;
            cur = p;
            if d >= 8 {
                break; // cycle guard: the data is at most 4 deep
            }
        }
        d
    };
    let mut group: HashMap<BlockId, usize> = HashMap::new();
    let mut n_groups = 0;
    for &vid in &visible_idx {
        let b = ws.entry(vid).block;
        if !is_block_entry_block(ws, b) {
            continue;
        }
        if anom_children.get(&b).is_none_or(Vec::is_empty) {
            continue; // standalone block: no group, no tint.
        }
        if depth_of(b) > 1 {
            continue; // deeper parent: inherits the family tint.
        }
        group.entry(b).or_insert(n_groups);
        n_groups += 1;
    }
    for &vid in &visible_idx {
        let b = ws.entry(vid).block;
        if !is_block_entry_block(ws, b) {
            continue;
        }
        // Insert for `b` (the visible block being tinted), NOT for the
        // unit found up the chain: every row reads its OWN block tint, so
        // the whole subtree must be filled with the family color.
        let mut cur = Some(b);
        while let Some(cb) = cur {
            if let Some(&g) = group.get(&cb) {
                block_tint
                    .entry(b)
                    .or_insert(c::GROUP_TINTS[g % c::GROUP_TINTS.len()]);
                break;
            }
            cur = anom_parent.get(&cb).copied();
        }
    }

    // Trader blocks (TRADERS page): the parent row and all the item child
    // rows share one tint (same model as the delegation rows). The
    // delegation loop above already tinted the blocks that carry a visible
    // delegation — keep it; otherwise take the next palette tint (screen
    // order).
    let mut n_trade = 0;
    for &vid in &visible_idx {
        let b = ws.entry(vid).block;
        if is_trade_block(ws, b) && !block_tint.contains_key(&b) {
            block_tint.insert(b, c::GROUP_TINTS[n_trade % c::GROUP_TINTS.len()]);
            n_trade += 1;
        }
    }

    Derived {
        visible_idx,
        block_entries,
        delegation_tint,
        block_tint,
        dirty: ws.dirty_count(),
        // Counting only: does not build the `Vec<String>` of messages.
        n_err: ws
            .field_ids()
            .filter(|&f| {
                let f = ws.field(f);
                f.is_dirty() && !f.is_valid()
            })
            .count(),
        cat_counts: ws.category_counts(st.view.file_filter),
    }
}
