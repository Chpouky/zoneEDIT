//! Status bar (bottom) + validation errors panel.

use egui::{pos2, Align, Align2, FontId, Layout, RichText, Sense, Stroke};

use crate::domain::Workspace;
use crate::ui::metrics::{FONT_BODY, STATUS_H};
use crate::ui::state::{Action, Derived, EditorState, Modal};
use crate::ui_kit::c;

/// Status bar (32 px). To be shown **before** the central panel: a
/// bottom panel shown afterwards does not shrink the space already allocated to the
/// central.
pub(crate) fn draw_statusbar(ctx: &egui::Context, st: &mut EditorState, derived: &Derived) {
    egui::TopBottomPanel::bottom("statusbar")
        .exact_height(STATUS_H)
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            ui.painter().rect_filled(rect, 0.0, c::PANEL);
            // Top border: drawn by the egui fork (show_separator_line).
            // Previous frame time (plan 4.1: measure before optimizing;
            // if it's smooth, we stop there) — to the right of the bar.
            let frame_galley = ctx.fonts(|f| {
                f.layout_no_wrap(
                    format!("frame {:.1} ms", st.frame_ms),
                    FontId::proportional(FONT_BODY),
                    c::TEXT_MUTED,
                )
            });
            let frame_w = frame_galley.rect.width();
            // The galley is laid out top-left: recompute its position for a
            // right/center alignment.
            ui.painter().galley(
                pos2(
                    rect.right() - 12.0 - frame_w,
                    rect.center().y - frame_galley.rect.height() / 2.0,
                ),
                frame_galley,
                c::TEXT_MUTED,
            );
            // Ephemeral message, to the left of the indicator (it clears itself
            // after a few seconds — see the top of `show_editor`).
            if let Some((ok, text)) = st.msg.as_ref() {
                ui.painter().text(
                    pos2(rect.right() - 24.0 - frame_w, rect.center().y),
                    Align2::RIGHT_CENTER,
                    text,
                    FontId::proportional(FONT_BODY),
                    if *ok { c::TEXT_DIM } else { c::DANGER_FG },
                );
            }
            ui.horizontal_centered(|ui| {
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 20.0;
                    ui.colored_label(
                        c::TEXT_DIM,
                        format!(
                            "{} generators · {} entries",
                            derived.block_entries.len(),
                            derived.visible_idx.len(),
                        ),
                    );
                    ui.colored_label(
                        c::MODIFIED_FG,
                        format!("{} pending change(s)", derived.dirty),
                    );
                    if derived.n_err > 0 {
                        // Clickable: opens/closes the detailed errors panel.
                        if ui
                            .add(
                                egui::Label::new(
                                    RichText::new(format!("{} error(s) ▾", derived.n_err))
                                        .color(c::DANGER_FG),
                                )
                                .sense(Sense::click()),
                            )
                            .clicked()
                        {
                            st.push_action(Action::ShowErrors);
                        }
                    } else if matches!(st.modal, Some(Modal::Errors)) {
                        st.modal = None;
                    }
                });
            });
        });
}

/// Validation errors panel (above the status bar).
/// Displayed when the user clicks on "N error(s)" in the status bar:
/// the list of the detailed messages (file, line, reason).
pub(crate) fn draw_errors_panel(ctx: &egui::Context, ws: &mut Workspace, st: &mut EditorState) {
    if !matches!(st.modal, Some(Modal::Errors)) {
        return;
    }
    let errors: Vec<String> = ws
        .validation_errors()
        .iter()
        .map(|e| e.to_string())
        .collect();
    if errors.is_empty() {
        st.modal = None;
        return;
    }
    let h = (errors.len().min(5) as f32 * 18.0) + 46.0;
    egui::TopBottomPanel::bottom("errbar")
        .exact_height(h)
        .show_separator_line(false) // we draw our own red border
        .show(ctx, |ui| {
            ui.painter().rect_filled(ui.max_rect(), 0.0, c::PANEL);
            ui.painter().line_segment(
                [ui.max_rect().left_top(), ui.max_rect().right_top()],
                Stroke::new(1.0_f32, c::DANGER_FG),
            );
            ui.horizontal(|ui| {
                ui.add_space(12.0);
                ui.label(
                    RichText::new(format!("{} validation error(s):", errors.len()))
                        .size(12.0)
                        .strong()
                        .color(c::DANGER_FG),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button("×").clicked() {
                        st.modal = None;
                    }
                });
            });
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_space(4.0);
                    for e in &errors {
                        ui.label(
                            RichText::new(e.clone())
                                .size(11.0)
                                .monospace()
                                .color(c::TEXT),
                        );
                    }
                });
        });
}
