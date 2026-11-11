//! Left sidebar: section dropdown ("ITEM SPAWNS"…) + categories of the
//! selected section (groups + counters). The groups and categories come
//! from the configuration (`[[categories]]`, fields `group` and
//! `section`), no hard-coded lists.

use egui::{vec2, Align, Layout, RichText};

use crate::config::CategoryId;
use crate::domain::Workspace;
use crate::ui::metrics::{FONT_BODY, SEARCH_W, SIDEBAR_W};
use crate::ui::state::{Derived, EditorState};
use crate::ui::widgets::{anchored_popup, dropdown, menu_row, PopupSide};
use crate::ui_kit::{c, nav_item};

pub(crate) fn draw_sidebar(
    ctx: &egui::Context,
    ws: &Workspace,
    st: &mut EditorState,
    derived: &Derived,
) {
    egui::SidePanel::left("sidebar")
        .exact_width(SIDEBAR_W)
        .resizable(false)
        .frame(egui::Frame::new().fill(c::PANEL))
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            ui.indent("side_panel", |ui| {
                // ---- Section selector (dropdown) --------------------------------
                // Sections come from the configuration (order of first
                // appearance among the categories).
                ui.add_space(12.0);
                let sections = ws.config().sections();
                if st.view.section >= sections.len() {
                    st.view.section = sections.len().saturating_sub(1);
                }
                let current = sections
                    .get(st.view.section)
                    .map(String::as_str)
                    .unwrap_or(sections.first().map(String::as_str).unwrap_or(""));
                let r = dropdown(ui, current, SEARCH_W);
                if r.clicked() {
                    st.section_menu = !st.section_menu;
                    st.modal = None;
                }
                if st.section_menu {
                    let res =
                        anchored_popup(ctx, "section_menu", r.rect, PopupSide::Below, 0.0, |ui| {
                            ui.set_width(r.rect.width());
                            for (i, s) in sections.iter().enumerate() {
                                if menu_row(ui, s, i == st.view.section) {
                                    st.view.section = i;
                                    // The selected category must belong to the
                                    // section: otherwise select its first one.
                                    if !ws
                                        .config()
                                        .categories_of_section(s)
                                        .contains(&st.view.category)
                                    {
                                        if let Some(&first) =
                                            ws.config().categories_of_section(s).first()
                                        {
                                            st.view.category = first;
                                        }
                                    }
                                    st.section_menu = false;
                                }
                            }
                        });
                    if res.clicked_outside {
                        st.section_menu = false;
                    }
                }

                // ---- Categories of the selected section --------------------------
                // Contextual to the "File" filter of the topbar: with a file
                // selected, only the categories present in that file
                // are shown (and with their counters limited to that file).
                // Categories in a scrollable area: whatever the
                // window height, no group is truncated.
                ui.add_space(14.0);
                let cats_h = ui.available_height();
                let cats_rect =
                    egui::Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), cats_h));
                let mut cats_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(cats_rect)
                        .layout(Layout::top_down(Align::LEFT)),
                );
                {
                    let ui = &mut cats_ui;
                    egui::ScrollArea::vertical()
                        .id_salt(egui::Id::new("side_cats"))
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            // The section is resolved AFTER the dropdown: the
                            // user may have switched it this frame, and the
                            // list must already reflect it.
                            let current = sections
                                .get(st.view.section)
                                .map(String::as_str)
                                .unwrap_or(sections.first().map(String::as_str).unwrap_or(""));
                            let section_ids: std::collections::HashSet<CategoryId> = ws
                                .config()
                                .categories_of_section(current)
                                .into_iter()
                                .collect();
                            // `category_groups()` orders the groups by first
                            // appearance; keep only the categories of the
                            // current section.
                            let groups: Vec<(&str, Vec<CategoryId>)> = ws
                                .config()
                                .category_groups()
                                .into_iter()
                                .map(|(g, ids)| {
                                    (
                                        g,
                                        ids.into_iter()
                                            .filter(|id| section_ids.contains(id))
                                            .collect::<Vec<_>>(),
                                    )
                                })
                                .filter(|(_, ids)| !ids.is_empty())
                                .collect();
                            let counts = &derived.cat_counts;
                            let file_only = st.view.file_filter.is_some();
                            for (group, cats) in &groups {
                                // Single file: hide the groups without any category.
                                let shown: Vec<CategoryId> = cats
                                    .iter()
                                    .copied()
                                    .filter(|&cat| {
                                        !file_only
                                            || counts.iter().any(|(cc, n)| *cc == cat && *n > 0)
                                    })
                                    .collect();
                                if shown.is_empty() {
                                    continue;
                                }
                                ui.add_space(10.0);
                                ui.label(
                                    RichText::new(*group)
                                        .size(FONT_BODY)
                                        .strong()
                                        .color(c::TEXT_STRONG),
                                );
                                ui.add_space(2.0);
                                for cat in shown {
                                    let spec = ws.config().category(cat);
                                    let n = counts
                                        .iter()
                                        .find(|(cc, _)| *cc == cat)
                                        .map(|(_, n)| *n)
                                        .unwrap_or(0);
                                    if file_only && n == 0 {
                                        continue;
                                    }
                                    if nav_item(ui, &spec.short, n, st.view.category == cat)
                                        .clicked()
                                    {
                                        st.view.category = cat;
                                    }
                                }
                            }
                            ui.add_space(10.0);
                        });
                }
                ui.advance_cursor_after_rect(cats_rect);
            });
            // Right border: drawn by the egui fork (show_separator_line).
        });
}
