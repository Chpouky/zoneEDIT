//! Small shared widgets: buttons, dropdowns, menus, checkboxes.

use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Response, RichText,
    Sense, Stroke, StrokeKind, Ui,
};

use std::collections::HashSet;

use crate::domain::{EntryId, FieldId, Workspace};
use crate::ui::metrics::{
    BTN_MIN_W, CB_BOX, CB_HIT, CELL_GAP, COLS_RIGHT_MARGIN, COL_PAD_X, COL_W_MIN, CTRL_H,
    CTRL_PAD_X, DROP_PAD, FONT_BODY, FONT_BUTTON, FONT_SMALL, ITEM_NAME_X, NAME_MIN_W,
    SEARCH_CLEAR_BTN, SEARCH_CLEAR_GAP, SEARCH_CLEAR_X_ARM, SEARCH_CLEAR_X_STROKE, SEARCH_H,
    SEARCH_PAD_X,
};
use crate::ui::state::EditorState;
use crate::ui_kit::c;

// ---------------------------------------------------------------------------
// Layout helpers
// ---------------------------------------------------------------------------

pub(crate) fn bar_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(c::PANEL)
        .stroke(Stroke::new(1.0_f32, c::BORDER))
}

/// Chrome of a table row: allocates the row rect, stacks the base fill
/// (selection / header background) under the optional tint, and draws the
/// 1 px bottom separator line. Returns the row rect.
pub(crate) fn row_frame(
    ui: &mut Ui,
    row_w: f32,
    h: f32,
    base: Option<Color32>,
    tint: Option<Color32>,
) -> Rect {
    let rect = ui
        .allocate_rect(
            Rect::from_min_size(ui.cursor().min, vec2(row_w, h)),
            Sense::hover(),
        )
        .rect;
    let painter = ui.painter_at(rect);
    if let Some(f) = base {
        painter.rect_filled(rect, 0.0, f);
    }
    if let Some(t) = tint {
        // Tint painted AFTER the base fill: it stays visible even on a
        // selected row (alpha ≈ 10%).
        painter.rect_filled(rect, 0.0, t);
    }
    painter.line_segment(
        [
            pos2(rect.left(), rect.bottom()),
            pos2(rect.right(), rect.bottom()),
        ],
        Stroke::new(1.0_f32, c::ROW_LINE),
    );
    rect
}

pub(crate) fn dim12(ui: &mut Ui, t: &str) {
    ui.label(RichText::new(t).size(FONT_BUTTON).color(c::TEXT_MUTED));
}

pub(crate) fn btn(ui: &mut Ui, t: &str) -> Response {
    ui.add(egui::Button::new(RichText::new(t).size(FONT_BUTTON)).min_size(vec2(BTN_MIN_W, CTRL_H)))
}

/// Painted dropdown (mockup): control background, arrow, width ≥ min_w.
pub(crate) fn dropdown(ui: &mut Ui, text: &str, min_w: f32) -> Response {
    let galley =
        ui.painter()
            .layout_no_wrap(text.to_string(), FontId::proportional(FONT_BUTTON), c::TEXT);
    let w = (galley.size().x + DROP_PAD).max(min_w);
    let (rect, resp) = ui.allocate_exact_size(vec2(w, CTRL_H), Sense::click());
    let p = ui.painter();
    p.rect(
        rect,
        4.0,
        if resp.hovered() {
            c::HOVER
        } else {
            c::CONTROL_BG
        },
        Stroke::new(1.0_f32, c::BORDER_CTRL),
        StrokeKind::Inside,
    );
    p.text(
        rect.left_center() + vec2(CTRL_PAD_X, 0.0),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(FONT_BUTTON),
        c::TEXT,
    );
    let m = egui::pos2(rect.right() - 16.0, rect.center().y);
    let st = Stroke::new(1.4_f32, c::TEXT_MUTED);
    p.line_segment([m + vec2(-4.0, -2.0), m + vec2(0.0, 2.0)], st);
    p.line_segment([m + vec2(0.0, 2.0), m + vec2(4.0, -2.0)], st);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Search field ("Search item SID…"): input background, placeholder,
/// accent border on focus, clear button. Drawn at the current cursor
/// position with a fixed width (filter bar, right-aligned).
pub(crate) fn draw_search_field(ui: &mut Ui, st: &mut EditorState, w: f32) {
    // `allocate_exact_size` positions the field according to the current
    // layout (the filter bar uses right_to_left, so the field ends up
    // right-aligned) and advances the cursor.
    let (rect, _) = ui.allocate_exact_size(vec2(w, SEARCH_H), Sense::hover());
    ui.painter().rect(
        rect,
        4.0,
        c::INPUT_BG,
        Stroke::new(1.0_f32, c::BORDER_CTRL),
        StrokeKind::Inside,
    );
    // Content of the field: child left_to_right. The cross axis
    // (Align::Center) centers each element vertically —
    // it's the layout rule that centers, not an offset.
    // The TextEdit receives the EXACT height of its galley: a
    // TextEdit taller than its text draws the text at the top
    // of its box (it does not center itself), while the
    // parent centers the box on the cross axis.
    let font_id = FontId::proportional(FONT_BODY);
    let text_h = ui.fonts(|f| f.row_height(&font_id));
    let field_w = w - 2.0 * SEARCH_PAD_X - SEARCH_CLEAR_BTN - SEARCH_CLEAR_GAP;
    let mut search_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    search_ui.add_space(SEARCH_PAD_X);
    let resp = search_ui.add_sized(
        vec2(field_w, text_h),
        egui::TextEdit::singleline(&mut st.view.search)
            .hint_text("Search item SID…")
            .frame(false)
            .margin(egui::Margin::ZERO)
            .font(font_id),
    );
    if resp.has_focus() {
        ui.painter().rect(
            rect,
            4.0,
            Color32::TRANSPARENT,
            Stroke::new(1.5_f32, c::ACCENT),
            StrokeKind::Inside,
        );
    }
    // "×" button: clears the search field. Dark gray when the field
    // is empty, normal otherwise (same states as the batch buttons
    // of the bottom bar, via `add_enabled`). The "×" glyph of the
    // font is not centered in its line box: the cross is therefore
    // drawn by the Painter, in two diagonal segments symmetric around
    // the exact center of the button. The interaction (hover/click/
    // enable) stays on the Button.
    search_ui.add_space(SEARCH_CLEAR_GAP);
    let clear_enabled = !st.view.search.is_empty();
    let clear_btn = search_ui.add_enabled(
        clear_enabled,
        egui::Button::new("")
            .frame(false)
            .min_size(vec2(SEARCH_CLEAR_BTN, SEARCH_CLEAR_BTN)),
    );
    if clear_btn.clicked() {
        st.view.search.clear();
    }
    {
        let c = clear_btn.rect.center();
        let color = if clear_enabled {
            search_ui.visuals().text_color()
        } else {
            search_ui.visuals().weak_text_color()
        };
        let stroke = Stroke::new(SEARCH_CLEAR_X_STROKE, color);
        let p = search_ui.painter();
        let d1 = vec2(SEARCH_CLEAR_X_ARM, SEARCH_CLEAR_X_ARM);
        let d2 = vec2(SEARCH_CLEAR_X_ARM, -SEARCH_CLEAR_X_ARM);
        p.line_segment([c - d1, c + d1], stroke);
        p.line_segment([c - d2, c + d2], stroke);
    }
}

/// Menu line (Area): hover #333, active = background + "active" on the right.
pub(crate) fn menu_row(ui: &mut Ui, label: &str, active: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), CTRL_H), Sense::click());
    if active {
        ui.painter().rect_filled(rect, 0.0, c::BORDER);
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, 0.0, c::HOVER);
    }
    ui.painter().text(
        rect.left_center() + vec2(CTRL_PAD_X, 0.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(FONT_BODY),
        if active { c::TEXT_STRONG } else { c::TEXT_DIM },
    );
    if active {
        ui.painter().text(
            rect.right_center() - vec2(CTRL_PAD_X, 0.0),
            Align2::RIGHT_CENTER,
            "active",
            FontId::proportional(FONT_SMALL),
            c::TEXT_MUTED,
        );
    }
    resp.clicked()
}

/// Small hand-drawn trash bin (14×14, centered on `c`): no
/// dependency on a glyph/emoji that might be missing in the font.
pub(crate) fn draw_trash(p: &egui::Painter, c: Pos2, col: Color32) {
    let s = 14.0;
    let m = c - vec2(s / 2.0, s / 2.0);
    let stroke = Stroke::new(1.4_f32, col);
    // Couvercle + anse
    p.line_segment([m + vec2(1.5, 2.6), m + vec2(12.5, 2.6)], stroke);
    p.add(egui::Shape::line(
        [
            m + vec2(5.2, 2.6),
            m + vec2(5.2, 1.0),
            m + vec2(8.8, 1.0),
            m + vec2(8.8, 2.6),
        ]
        .to_vec(),
        stroke,
    ));
    // Trapezoidal body + ribs
    p.add(egui::Shape::line(
        [
            m + vec2(3.4, 2.6),
            m + vec2(4.2, 13.0),
            m + vec2(9.8, 13.0),
            m + vec2(10.6, 2.6),
        ]
        .to_vec(),
        stroke,
    ));
    p.line_segment([m + vec2(6.1, 5.4), m + vec2(6.5, 10.4)], stroke);
    p.line_segment([m + vec2(7.9, 5.4), m + vec2(7.5, 10.4)], stroke);
}

pub(crate) fn menu_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(c::CONTROL_BG)
        .stroke(Stroke::new(1.0_f32, c::BORDER_CTRL))
        .corner_radius(CornerRadius::same(4))
}

/// Opening side of an anchored popup.
#[derive(Clone, Copy)]
pub(crate) enum PopupSide {
    /// Upward: the bottom of the popup is 8 px above the top of the anchor.
    Above,
    /// Downward: the top of the popup is 6 px below the bottom of the anchor.
    Below,
}

/// Result of drawing an anchored popup.
pub(crate) struct PopupResult {
    /// Real height of the frame (to be reinjected the next frame in case
    /// of upward opening).
    pub meas_h: f32,
    /// Click OUTSIDE the popup (and the anchor): closing convention
    /// of the app. The OPENING click is on the anchor and `any_click()` is
    /// still true at the opening frame (release event): the anchor
    /// is therefore excluded, otherwise a menu opened by click would close
    /// itself.
    pub clicked_outside: bool,
}

/// THE unique mold of the app's anchored popups (topbar Folder/File menus,
/// presets menu, batch popups): a full-screen transparent `Area` +
/// `menu_frame` in `fixed_pos`, null item spacing.
///
/// `height` (estimated height or measured the previous frame) is only used
/// for the upward opening: the popup must know its height
/// to place itself above the anchor.
pub(crate) fn anchored_popup(
    ctx: &egui::Context,
    id: &'static str,
    anchor: Rect,
    side: PopupSide,
    height: f32,
    content: impl FnOnce(&mut Ui),
) -> PopupResult {
    let pos = match side {
        PopupSide::Above => pos2(anchor.left(), (anchor.top() - height - 8.0).max(0.0)),
        PopupSide::Below => pos2(anchor.left(), anchor.bottom() + 6.0),
    };
    let mut result = PopupResult {
        meas_h: height,
        clicked_outside: false,
    };
    egui::Area::new(egui::Id::new(id))
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .show(ctx, |ctx| {
            let resp = menu_frame().show(ctx, |ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                content(ui);
            });
            result.meas_h = resp.response.rect.height().max(1.0);
            result.clicked_outside = ctx.input(|i| {
                let p = &i.pointer;
                if p.any_click() {
                    if let Some(pos) = p.interact_pos() {
                        !resp.response.rect.contains(pos) && !anchor.contains(pos)
                    } else {
                        false
                    }
                } else {
                    false
                }
            });
        });
    result
}

/// Non-interactive note line of a menu: same look as `menu_row`
/// (height 32 px, indent 12 px) but without hover background. The caller must
/// constrain the width (`ui.set_width`), as for `menu_row`.
pub(crate) fn menu_note(ui: &mut Ui, label: &str) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), CTRL_H), Sense::hover());
    ui.painter().text(
        rect.left_center() + vec2(CTRL_PAD_X, 0.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(FONT_BODY),
        c::TEXT_DIM,
    );
}

/// One value column of the table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Col {
    /// Center x of the column (the value box is centered on it).
    pub center: f32,
    /// Width of the column: fits its header label, at least `COL_W_MIN`.
    pub width: f32,
}

/// Geometry of the value zone: one column per editable field of the current
/// category, right-anchored on the right edge of the table content. The
/// content is at least as wide as the viewport; when the name zone or the
/// columns do not fit, it is wider and the table scrolls horizontally.
pub(crate) struct Cols {
    /// One column per field, in the order of `category.columns`.
    pub cols: Vec<Col>,
    /// Width of the table content (max of the viewport and of the width
    /// needed by the name zone + columns + right margin).
    pub content_w: f32,
}

/// Builds the per-column layout. Each column is as wide as its header
/// `labels[i]` (measured in `font`) plus padding, at least `COL_W_MIN`.
/// `viewport_w` describes the visible body of the table; the content grows
/// beyond the viewport when needed (horizontal scrolling).
///
/// The column centers are **body-local** (0 = left edge of the body) so
/// they scroll with the content: rows add their own left edge (see
/// `col_at`), the sticky header adds its left edge minus the horizontal
/// scroll offset.
pub(crate) fn col_layout(
    painter: &Painter,
    viewport_w: f32,
    labels: &[String],
    font: &FontId,
) -> Cols {
    let widths: Vec<f32> = labels
        .iter()
        .map(|label| {
            let w = painter
                .layout_no_wrap(label.clone(), font.clone(), Color32::WHITE)
                .size()
                .x;
            (w + 2.0 * COL_PAD_X).max(COL_W_MIN)
        })
        .collect();
    let n = widths.len();
    let span: f32 = widths.iter().sum::<f32>() + CELL_GAP * n.saturating_sub(1) as f32;
    // Left (name) zone + columns + right margin (row reset button).
    let needed = ITEM_NAME_X + NAME_MIN_W + span + COLS_RIGHT_MARGIN;
    let content_w = viewport_w.max(needed);
    let mut cols = Vec::with_capacity(n);
    let mut x = content_w - COLS_RIGHT_MARGIN;
    for i in (0..n).rev() {
        cols.push(Col {
            center: x - widths[i] / 2.0,
            width: widths[i],
        });
        x -= widths[i] + CELL_GAP;
    }
    cols.reverse();
    Cols { cols, content_w }
}

/// Body-local column → row coordinates: the row `rect` already carries the
/// scroll offsets, the `Cols` centers are relative to the body.
pub(crate) fn col_at(col: &Col, row_left: f32) -> Col {
    Col {
        center: row_left + col.center,
        width: col.width,
    }
}

/// Truncates `text` with an ellipsis so its rendered width fits `max_w`.
/// Used for entry names in the name zone, which has a bounded width; value
/// column labels are never truncated (the columns are sized to fit them).
pub(crate) fn fit_label(painter: &Painter, text: &str, font: &FontId, max_w: f32) -> String {
    if painter
        .layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE)
        .size()
        .x
        <= max_w
    {
        return text.to_string();
    }
    let mut t = text.to_string();
    while !t.is_empty()
        && painter
            .layout_no_wrap(format!("{t}\u{2026}"), font.clone(), Color32::WHITE)
            .size()
            .x
            > max_w
    {
        t.pop();
    }
    format!("{t}\u{2026}")
}

// ---------------------------------------------------------------------------
// Selection checkboxes (rows, ranks, blocks)
// ---------------------------------------------------------------------------

/// State of a "select all" checkbox.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CbState {
    Off,
    Partial,
    Checked,
}

/// Draws a checkbox centered on `center` (13px box) and returns the
/// hit zone (18px). App style: gray outline (empty), accent dash
/// (partial), accent fill + check (all checked).
pub(crate) fn draw_checkbox(painter: &egui::Painter, center: Pos2, state: CbState) -> Rect {
    let hit = Rect::from_center_size(center, vec2(CB_HIT, CB_HIT));
    let sq = Rect::from_center_size(center, vec2(CB_BOX, CB_BOX));
    match state {
        CbState::Off => {
            painter.rect(
                sq,
                2.0,
                Color32::TRANSPARENT,
                Stroke::new(1.2f32, c::CB_BORDER),
                egui::StrokeKind::Middle,
            );
        }
        CbState::Partial => {
            painter.rect(
                sq,
                2.0,
                Color32::TRANSPARENT,
                Stroke::new(1.4f32, c::ACCENT),
                egui::StrokeKind::Middle,
            );
            painter.line_segment(
                [
                    pos2(sq.left() + 3.0, center.y),
                    pos2(sq.right() - 3.0, center.y),
                ],
                Stroke::new(1.8f32, c::ACCENT),
            );
        }
        CbState::Checked => {
            painter.rect_filled(sq, 2.0, c::ACCENT);
            painter.line_segment(
                [
                    pos2(sq.left() + 2.5, center.y + 0.5),
                    pos2(sq.center().x - 1.5, sq.bottom() - 3.0),
                ],
                Stroke::new(1.8f32, c::ON_ACCENT),
            );
            painter.line_segment(
                [
                    pos2(sq.center().x - 1.5, sq.bottom() - 3.0),
                    pos2(sq.right() - 2.5, sq.top() + 3.0),
                ],
                Stroke::new(1.8f32, c::ON_ACCENT),
            );
        }
    }
    hit
}

/// "Select all" state for a set of row keys.
pub(crate) fn cb_state_for(keys: &[FieldId], batch: &HashSet<FieldId>) -> CbState {
    if keys.is_empty() {
        return CbState::Off;
    }
    let sel = keys.iter().filter(|k| batch.contains(k)).count();
    if sel == keys.len() {
        CbState::Checked
    } else if sel > 0 {
        CbState::Partial
    } else {
        CbState::Off
    }
}

/// Selection keys (1st field) of the visible item rows of a block, even
/// folded, possibly restricted to a rank (group label).
/// `entries` already contains only the visible entries of this block
/// (`Derived::block_entries`).
pub(crate) fn selectable_row_keys(
    ws: &Workspace,
    entries: &[EntryId],
    rank: Option<&str>,
) -> Vec<FieldId> {
    entries
        .iter()
        .filter_map(|&id| {
            let e = ws.entry(id);
            match rank {
                Some(r) => (e.group.as_deref() == Some(r))
                    .then(|| e.fields.first().copied())
                    .flatten(),
                None => e.fields.first().copied(),
            }
        })
        .collect()
}

/// Click on the "select all" checkbox: all checked → uncheck all;
/// otherwise → check all (without duplicates).
pub(crate) fn toggle_select_all(st: &mut EditorState, keys: &[FieldId]) {
    let all = !keys.is_empty() && keys.iter().all(|k| st.selection.batch.contains(k));
    if all {
        for k in keys {
            st.selection.batch.remove(k);
        }
    } else {
        for k in keys {
            st.selection.batch.insert(*k);
        }
    }
}

/// Label of the preset dropdown.
pub(crate) fn preset_label(st: &EditorState) -> String {
    if st.presets.choice == 0 {
        "Vanilla".into()
    } else {
        st.presets
            .list
            .get(st.presets.choice - 1)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "Vanilla".into())
    }
}
