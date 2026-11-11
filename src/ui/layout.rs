//! Row layout: the left-side geometry of every row of the main table.
//!
//! All the row types (item, anomaly child, block header, rank sub-dropdown,
//! set/caliber) go through `RowLayout` + `FoldTri`, so the columns
//! (checkbox, fold triangle, icon, name, set label) stay aligned from one
//! section to another. Adding a row type = adding a constructor
//! here, not a new pile of magic numbers in `rows.rs` / `table.rs`.

use egui::{pos2, vec2, Painter, Pos2, Rect, Stroke};

use crate::ui::metrics::{
    CB_CENTER_X, CB_HIT, FONT_BODY, GEN_NAME_X, GEN_TRI_X, ICON_X, ITEM_NAME_X, LINE_FONT,
    LINE_X_GAP, RANK_NAME_X, RANK_TRI_X, ROW_H, ROW_H_TALL, SET_LABEL_X, TAG_H, TAG_TOP, TAG_W,
    TAG_X_GAP,
};
use crate::ui_kit::c;

// ---------------------------------------------------------------------------
// Fold triangle
// ---------------------------------------------------------------------------

/// Fold/unfold triangle of a header row (block, rank, anomaly parent).
///
/// One single geometry for every triangle of the app: a right-pointing
/// triangle when collapsed, a down-pointing one when expanded, centered on
/// the row. `x` is the base (left) edge, offset from the left edge of the
/// row; `hit` is the click zone (same coordinates).
#[derive(Clone, PartialEq)]
pub(crate) struct FoldTri {
    /// Base x of the triangle (offset from the left edge of the row).
    pub x: f32,
    /// Click zone: x range, offset from the left edge of the row.
    pub hit: std::ops::Range<f32>,
}

impl FoldTri {
    pub(crate) fn new(x: f32, hit: std::ops::Range<f32>) -> Self {
        Self { x, hit }
    }

    /// Vertices of the triangle centered on `cy` (offsets from the left
    /// edge of the row).
    pub(crate) fn points(&self, cy: f32, collapsed: bool) -> [Pos2; 3] {
        if collapsed {
            [
                Pos2::new(self.x, cy - 4.5),
                Pos2::new(self.x, cy + 4.5),
                Pos2::new(self.x + 5.5, cy),
            ]
        } else {
            [
                Pos2::new(self.x - 2.5, cy - 2.5),
                Pos2::new(self.x + 6.5, cy - 2.5),
                Pos2::new(self.x + 2.0, cy + 3.0),
            ]
        }
    }

    /// Paints the triangle in the row (`row` = full row rect).
    pub(crate) fn paint(&self, p: &Painter, row: Rect, collapsed: bool) {
        let pts = self
            .points(row.center().y, collapsed)
            .map(|pt| Pos2::new(row.left() + pt.x, pt.y));
        p.add(egui::Shape::convex_polygon(
            pts.to_vec(),
            c::TEXT_DIM,
            Stroke::NONE,
        ));
    }

    /// `true` if the pointer (absolute x) is inside the click zone.
    pub(crate) fn hit_test(&self, row: Rect, pointer_x: f32) -> bool {
        self.hit.contains(&(pointer_x - row.left()))
    }
}

// ---------------------------------------------------------------------------
// Row layout
// ---------------------------------------------------------------------------

/// Left-side layout of a table row: its height + the x positions (offsets
/// from the left edge of the row) of the checkbox, the fold triangle, the
/// icon and the name. The value cells are NOT part of the layout: they are
/// right-anchored columns (`widgets::col_centers`), shared by every row.
#[derive(Clone, PartialEq)]
pub(crate) struct RowLayout {
    /// Height of the row.
    pub h: f32,
    /// Center x of the checkbox (`None` = no checkbox).
    pub cb: Option<f32>,
    /// Fold triangle (`None` = no triangle).
    pub tri: Option<FoldTri>,
    /// Left edge of the icon (`None` = no icon).
    pub icon_x: Option<f32>,
    /// Left edge of the name.
    pub name_x: f32,
    /// Left edge of the itemset label (set/caliber rows only).
    pub set_label_x: Option<f32>,
}

impl RowLayout {
    /// Item row: checkbox + icon + name. The reference layout of the table
    /// — every other row aligns its columns on it.
    pub(crate) fn item() -> Self {
        Self {
            h: ROW_H,
            cb: Some(CB_CENTER_X),
            tri: None,
            icon_x: Some(ICON_X),
            name_x: ITEM_NAME_X,
            set_label_x: None,
        }
    }

    /// Block ("generator") header row: checkbox + fold triangle (in the icon
    /// slot) + name, no icon.
    pub(crate) fn generator() -> Self {
        Self {
            h: ROW_H_TALL,
            cb: Some(CB_CENTER_X),
            tri: Some(FoldTri::new(GEN_TRI_X, 34.0..58.0)),
            icon_x: None,
            name_x: GEN_NAME_X,
            set_label_x: None,
        }
    }

    /// Rank sub-dropdown header row: checkbox + fold triangle (indented one
    /// level below the block triangle) + name, no icon.
    pub(crate) fn rank() -> Self {
        Self {
            h: ROW_H,
            cb: Some(CB_CENTER_X),
            tri: Some(FoldTri::new(RANK_TRI_X, 34.0..76.0)),
            icon_x: None,
            name_x: RANK_NAME_X,
            set_label_x: None,
        }
    }

    /// Set/caliber row: itemset label (or group label in the name column) +
    /// value cells, no checkbox and no triangle.
    pub(crate) fn set_row() -> Self {
        Self {
            h: ROW_H,
            cb: None,
            tri: None,
            icon_x: None,
            name_x: RANK_NAME_X,
            set_label_x: Some(SET_LABEL_X),
        }
    }

    /// Hit zone of the row checkbox (`None` = no checkbox).
    pub(crate) fn cb_rect(&self, row: Rect) -> Option<Rect> {
        self.cb.map(|x| {
            Rect::from_center_size(pos2(row.left() + x, row.center().y), vec2(CB_HIT, CB_HIT))
        })
    }

    /// Absolute position of the name (left, vertically centered).
    pub(crate) fn name_pos(&self, row: Rect) -> Pos2 {
        pos2(row.left() + self.name_x, row.center().y)
    }

    /// Absolute position of the set label (left, vertically centered).
    pub(crate) fn set_label_pos(&self, row: Rect) -> Option<Pos2> {
        self.set_label_x
            .map(|x| pos2(row.left() + x, row.center().y))
    }

    /// Absolute rect of the icon, fitted to the given (scaled) size.
    pub(crate) fn icon_rect(&self, row: Rect, size: egui::Vec2) -> Option<Rect> {
        self.icon_x
            .map(|x| Rect::from_min_size(pos2(row.left() + x, row.center().y - size.y / 2.0), size))
    }

    /// Paints the fold triangle, if any.
    pub(crate) fn paint_tri(&self, p: &Painter, row: Rect, collapsed: bool) {
        if let Some(tri) = &self.tri {
            tri.paint(p, row, collapsed);
        }
    }

    /// `true` if the pointer (absolute x) is inside the triangle zone.
    pub(crate) fn tri_hit(&self, row: Rect, pointer_x: f32) -> bool {
        self.tri
            .as_ref()
            .is_some_and(|t| t.hit_test(row, pointer_x))
    }
}

// ---------------------------------------------------------------------------
// Block header extras (name + "L{line}" + refs tag)
// ---------------------------------------------------------------------------

/// Paints the block header label: the name (strong) + the "L{line}" label
/// (muted, monospace) + the optional refs tag (child ui, natural size).
pub(crate) fn paint_block_label(
    p: &Painter,
    ui: &mut egui::Ui,
    row: Rect,
    name: &str,
    line: usize,
    refs: u32,
    show_refs: bool,
) {
    let cy = row.center().y;
    let name_w = p
        .text(
            pos2(row.left() + GEN_NAME_X, cy),
            egui::Align2::LEFT_CENTER,
            name.to_owned(),
            egui::FontId::proportional(FONT_BODY),
            c::TEXT_STRONG,
        )
        .width();
    let lx = row.left() + GEN_NAME_X + name_w + LINE_X_GAP;
    let lin_w = p
        .text(
            pos2(lx, cy),
            egui::Align2::LEFT_CENTER,
            format!("L{line}"),
            egui::FontId::monospace(LINE_FONT),
            c::TEXT_MUTED,
        )
        .width();
    if show_refs && refs > 0 {
        let area = Rect::from_min_size(
            pos2(lx + lin_w + TAG_X_GAP, row.top() + TAG_TOP),
            vec2(TAG_W, TAG_H),
        );
        let mut ch = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(area)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        crate::ui_kit::tag(&mut ch, &format!("{refs} refs"), false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_are_aligned_across_row_types() {
        // The checkbox column is common to all the rows that have one.
        for rl in [RowLayout::item(), RowLayout::generator(), RowLayout::rank()] {
            assert_eq!(rl.cb, Some(CB_CENTER_X));
        }
        // The headers (no icon) put their triangle in the icon slot or
        // indented below it, and their name stays left of the item names.
        let (gen, rank) = (RowLayout::generator(), RowLayout::rank());
        assert_eq!(gen.tri.unwrap().x, ICON_X);
        assert!(rank.tri.unwrap().x > ICON_X);
        assert!(gen.name_x < ITEM_NAME_X);
        assert!(rank.name_x < ITEM_NAME_X);
        assert_eq!(rank.name_x, RowLayout::set_row().name_x);
    }

    #[test]
    fn fold_triangle_geometry() {
        let tri = FoldTri::new(GEN_TRI_X, 34.0..58.0);
        let row = Rect::from_min_size(pos2(0.0, 100.0), vec2(600.0, ROW_H_TALL));
        let cy = row.center().y;
        // Collapsed: right-pointing, base vertical on `x`.
        let c = tri.points(cy, true);
        assert_eq!((c[0].x, c[1].x), (GEN_TRI_X, GEN_TRI_X));
        assert!(c[2].x > GEN_TRI_X);
        // Expanded: down-pointing.
        let e = tri.points(cy, false);
        assert_eq!((e[0].y, e[1].y), (cy - 2.5, cy - 2.5));
        assert!(e[2].y > cy);
        // Hit zone.
        assert!(tri.hit_test(row, row.left() + 40.0));
        assert!(!tri.hit_test(row, row.left() + 10.0));
    }
}
