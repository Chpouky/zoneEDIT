//! Named layout metrics — single source of the layout literals.
//!
//! Every pixel value used by the table rows and the bars lives here, so the
//! rows of every section (item spawns, anomalies, …) share the same geometry
//! and the tests can pin it down.

// ---------------------------------------------------------------------------
// Bars
// ---------------------------------------------------------------------------

/// Height of the top bar (logo + dropdowns).
pub(crate) const TOPBAR_H: f32 = 52.0;
/// Height of the filter bar (category + file + search).
pub(crate) const FILTER_H: f32 = 44.0;
/// Height of the sticky table header.
pub(crate) const HEADER_H: f32 = 30.0;
/// Height of the batch action bar ("Apply N").
pub(crate) const BATCH_H: f32 = 52.0;
/// Height of the status bar (bottom of the window).
pub(crate) const STATUS_H: f32 = 32.0;
/// Width of the left sidebar.
pub(crate) const SIDEBAR_W: f32 = 250.0;
/// Width of the search field (right side of the filter bar) and of the
/// sidebar file dropdown.
pub(crate) const SEARCH_W: f32 = 220.0;
/// Height of the search field.
pub(crate) const SEARCH_H: f32 = 32.0;
/// Left/right inner padding of the search field.
pub(crate) const SEARCH_PAD_X: f32 = 10.0;
/// Size of the search-clear "×" button.
pub(crate) const SEARCH_CLEAR_BTN: f32 = 20.0;
/// Gap between the entered text and the "×" button.
pub(crate) const SEARCH_CLEAR_GAP: f32 = 6.0;
/// Half-arm of the "×" cross (8 px in total — the size of the ink
/// of the "×" glyph of the 14 px font it replaces).
pub(crate) const SEARCH_CLEAR_X_ARM: f32 = 4.0;
/// Thickness of the strokes of the "×" cross.
pub(crate) const SEARCH_CLEAR_X_STROKE: f32 = 1.4;

// ---------------------------------------------------------------------------
// Table rows
// ---------------------------------------------------------------------------

/// Height of a group / item line.
pub(crate) const ROW_H: f32 = 36.0;
/// Height of a block / rank line (slightly taller).
pub(crate) const ROW_H_TALL: f32 = 40.0;
/// Target height of the item icons: VERTICAL fit (the native ratio is
/// preserved, a horizontal weapon therefore keeps its shape).
pub(crate) const ICON_H: f32 = 28.0;
/// Width of the icon box (the names stay aligned whatever the icon).
pub(crate) const ICON_BOX_W: f32 = 56.0;
/// Left edge of the icon box (offset from the left edge of the row).
pub(crate) const ICON_X: f32 = 40.0;
/// Position of the name of the item rows. Fixed for ALL the rows (even those
/// without icon): the names must stay perfectly aligned from one row to
/// another, whatever the width of the icon.
pub(crate) const ITEM_NAME_X: f32 = ICON_X + ICON_BOX_W + 10.0;
/// Base x of the warning triangle on orphan rows ("not found" delegations).
pub(crate) const ORPHAN_TRI_X: f32 = 18.0;
/// Monospace font size of the "L{line}" / "not found" labels.
pub(crate) const LINE_FONT: f32 = 11.0;
/// Height of the standard controls (buttons, dropdowns, menu rows, search).
pub(crate) const CTRL_H: f32 = 32.0;
/// Minimum width of a neutral button.
pub(crate) const BTN_MIN_W: f32 = 44.0;
/// Horizontal padding of a dropdown (text → arrow zone).
pub(crate) const DROP_PAD: f32 = 44.0;
/// Horizontal inset of the control content (arrow, clear button).
pub(crate) const CTRL_PAD_X: f32 = 12.0;
/// Box of a value cell (display and editor).
pub(crate) const CELL_BOX_W: f32 = 64.0;
pub(crate) const CELL_BOX_H: f32 = 26.0;
/// Corner radius of the cell box.
pub(crate) const CELL_RADIUS: u8 = 3;
/// Font size of the cell value (monospace).
pub(crate) const CELL_FONT: f32 = 12.0;

// ---------------------------------------------------------------------------
// Left slots of the "header" rows (fold triangle / name / set label)
// ---------------------------------------------------------------------------

/// Base x of the fold triangle of a block ("generator") header row. It sits
/// in the icon slot, where item rows show their icon.
pub(crate) const GEN_TRI_X: f32 = ICON_X;
/// Left edge of the name on a block header row (right of the triangle).
pub(crate) const GEN_NAME_X: f32 = 60.0;
/// Gap between the block name and its "L{line}" label.
pub(crate) const LINE_X_GAP: f32 = 10.0;
/// Gap between the "L{line}" label and the refs tag area.
pub(crate) const TAG_X_GAP: f32 = 12.0;
/// Width / height / top offset of the refs tag area in a block header.
pub(crate) const TAG_W: f32 = 500.0;
pub(crate) const TAG_H: f32 = 20.0;
pub(crate) const TAG_TOP: f32 = 11.0;
/// Base x of the fold triangle of a RANK sub-dropdown row (indented one
/// level below the block triangle).
pub(crate) const RANK_TRI_X: f32 = 58.0;
/// Left edge of the name on a rank header row (right of the triangle).
pub(crate) const RANK_NAME_X: f32 = 78.0;
/// Left edge of the itemset label on a set/caliber row (indented below the
/// rank name).
pub(crate) const SET_LABEL_X: f32 = 96.0;
/// Gap between the item name and its small rank label.
pub(crate) const RANK_LABEL_GAP: f32 = 8.0;
/// Horizontal indent per hierarchy level in a "block = entry" tree
/// (anomaly variant, weapon family template, gun). A child at depth *d*
/// carries ONE "↳" marker and its name is shifted right by *d* × this
/// constant — the offset, not the number of arrows, encodes the depth
/// (see `draw_item_row`).
pub(crate) const HIER_INDENT: f32 = 14.0;
/// Inset of the row-reset button center from the right edge of the row.
pub(crate) const ROW_RESET_INSET: f32 = 26.0;
// ---------------------------------------------------------------------------
// Checkbox (one column, common to all levels)
// ---------------------------------------------------------------------------

/// Centre X of the row checkbox (offset from the left edge of the row).
pub(crate) const CB_CENTER_X: f32 = 22.0;
/// Checkbox: size of the box (px).
pub(crate) const CB_BOX: f32 = 13.0;
/// Checkbox: hit zone (px) — the clickable/hoverable square around the box,
/// used by every checkbox in the app (rows, filter bar).
pub(crate) const CB_HIT: f32 = 18.0;

// ---------------------------------------------------------------------------
// Value cells (right-anchored columns, see `widgets::col_layout`)
// ---------------------------------------------------------------------------

/// Minimum width of a numeric column (the column widens to fit its label).
pub(crate) const COL_W_MIN: f32 = 72.0;
/// Horizontal padding added on each side of a column's measured label.
pub(crate) const COL_PAD_X: f32 = 8.0;
/// Horizontal spacing between columns.
pub(crate) const CELL_GAP: f32 = 8.0;
/// Minimum width of the name zone (left edge of the row → first column):
/// below it the table content grows wider than the window and the table
/// scrolls horizontally.
pub(crate) const NAME_MIN_W: f32 = 240.0;
/// Right margin of the cell zone: row reset button (20 px)
/// + right margin (16 px) + gap (8 px).
pub(crate) const COLS_RIGHT_MARGIN: f32 = 44.0;

// ---------------------------------------------------------------------------
// Fonts
// ---------------------------------------------------------------------------

/// Font size: small labels.
pub(crate) const FONT_SMALL: f32 = 10.5;
/// Font size: body text.
pub(crate) const FONT_BODY: f32 = 13.0;
/// Font size: buttons / inputs.
pub(crate) const FONT_BUTTON: f32 = 12.0;
