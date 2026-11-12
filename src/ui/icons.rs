//! Icons per item.
//!
//! The PNGs are embedded at compile time by `build.rs`
//! (files `assets/icons/<SID>.png`, extracted from `InventoryUIExports/`
//! via `match_icons.py`), native dimensions included. The decoding is
//! delegated to the `egui_extras` loader (installed in `main.rs`): the bytes
//! are decoded lazily at the first drawing then cached by
//! egui. The `bytes://` URI is unique per SID — it is the key of the texture
//! cache.

use std::collections::HashMap;
use std::sync::OnceLock;

include!(concat!(env!("OUT_DIR"), "/icons_table.rs"));

/// Rendering of an icon: source (cacheable by egui) + native dimensions.
pub struct ItemIcon {
    pub src: egui::ImageSource<'static>,
    pub size: egui::Vec2,
    /// True only when the SID (and its variant chain) resolved to the
    /// generic `_placeholder.png`. A blueprint SID resolves to the
    /// dedicated `_blueprint.png` icon: that is a REAL icon, not a
    /// placeholder.
    pub is_placeholder: bool,
}

type Table = HashMap<&'static str, (&'static [u8], u32, u32)>;

/// Contains `needle` (ASCII case-insensitive comparison), without
/// allocation.
fn has_substr_ignore_ascii_case(haystack: &str, needle: &str) -> bool {
    haystack.len() >= needle.len()
        && (0..=haystack.len() - needle.len())
            .any(|i| haystack[i..i + needle.len()].eq_ignore_ascii_case(needle))
}

static MAP: OnceLock<Table> = OnceLock::new();

fn map() -> &'static Table {
    MAP.get_or_init(|| {
        TABLE
            .iter()
            .map(|(sid, bytes, w, h)| (*sid, (*bytes, *w, *h)))
            .collect()
    })
}

/// Candidate names of an item icon, in priority order: the exact SID, then
/// the weapon-variant forms (the `_Player` / `_NPC` suffixes and the `Guard`
/// prefix stripped, e.g. `GuardGunPM_HG_NPC` → `GuardGunPM_HG` → `GunPM_HG`),
/// so the weapon rows reuse the base weapon icon.
fn icon_candidates(sid: &str) -> [Option<&str>; 4] {
    let stripped = ["_Player", "_NPC"]
        .iter()
        .find_map(|s| sid.strip_suffix(s))
        .unwrap_or(sid);
    [
        Some(sid),
        (stripped != sid).then_some(stripped),
        sid.strip_prefix("Guard"),
        stripped.strip_prefix("Guard"),
    ]
}

/// Rendering of the icon of an item SID.
///
/// Fallbacks (SIDs without exported icon):
/// - the weapon-variant name chain (`icon_candidates`) — one icon export
///   per base weapon, shared by its `_Player` / `_NPC` / `Guard` variants;
/// - SID containing "blueprint" (case-insensitive): dedicated icon
///   `_blueprint.png` (the 80 blueprint exports were identical);
/// - otherwise: the placeholder (`_placeholder.png`).
pub(crate) fn icon_for(sid: &str) -> ItemIcon {
    let (bytes, w, h, uri, is_placeholder) = icon_candidates(sid)
        .iter()
        .find_map(|cand| match cand {
            Some(c) => map()
                .get(*c)
                .copied()
                .map(|(b, w, h)| (b, w, h, format!("bytes://icons/{c}.png"), false)),
            None => None,
        })
        .unwrap_or_else(|| {
            let fallback = if has_substr_ignore_ascii_case(sid, "blueprint") {
                "_blueprint"
            } else {
                "_placeholder"
            };
            let (bytes, w, h) = map()
                .get(fallback)
                .copied()
                .unwrap_or_else(|| panic!("{fallback}.png missing in assets/icons"));
            (
                bytes,
                w,
                h,
                format!("bytes://icons/{fallback}.png"),
                fallback == "_placeholder",
            )
        });
    ItemIcon {
        src: egui::ImageSource::Bytes {
            uri: uri.into(),
            bytes: egui::load::Bytes::Static(bytes),
        },
        size: egui::vec2(w as f32, h as f32),
        is_placeholder,
    }
}

/// Pop-up displayed when the mouse is over an item icon: the icon at its
/// original size (placeholder included) on an opaque background (the PNGs are
/// transparent), in the `Foreground` layer (so not clipped by the scroll of
/// the table) and following the cursor. Disappears as soon as the hover ends
/// (redrawn each frame, only during the hover).
pub(crate) fn draw_tooltip(ui: &mut egui::Ui, icon: &ItemIcon, at: egui::Pos2) {
    use crate::ui_kit::c;
    let screen = ui.ctx().input(|i| i.screen_rect());
    let pad = 8.0;
    // Original size, capped to the window if the icon is larger.
    let max = screen.size() - egui::vec2(48.0, 48.0);
    let scale = (max.x / icon.size.x).min(max.y / icon.size.y).min(1.0);
    let icon_size = icon.size * scale;
    let size = icon_size + egui::vec2(2.0 * pad, 2.0 * pad);

    // Bottom right of the cursor; flipped to top left if overflowing.
    let mut min = at + egui::vec2(18.0, 18.0);
    if min.x + size.x > screen.max.x {
        min.x = (at.x - 18.0 - size.x).max(screen.min.x);
    }
    if min.y + size.y > screen.max.y {
        min.y = (at.y - 18.0 - size.y).max(screen.min.y);
    }
    min.x = min.x.min((screen.max.x - size.x).max(screen.min.x));
    min.y = min.y.min((screen.max.y - size.y).max(screen.min.y));

    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(egui::Rect::from_min_size(
                min + egui::vec2(pad, pad),
                icon_size,
            ))
            .layout(egui::Layout::top_down(egui::Align::LEFT))
            .layer_id(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("icon_tooltip"),
            )),
    );
    // The painter inherits the clip of the parent (scroll of the table): we lift the
    // clipping, otherwise the pop-up would be cut at the edges of the visible area.
    child.set_clip_rect(egui::Rect::EVERYTHING);
    child.painter().rect(
        egui::Rect::from_min_size(min, size),
        egui::CornerRadius::same(6),
        c::PANEL,
        egui::Stroke::new(1.0_f32, c::BORDER_CTRL),
        egui::StrokeKind::Inside,
    );
    child.add(egui::Image::new(icon.src.clone()).fit_to_exact_size(icon_size));
}
#[cfg(test)]
mod tests {
    use super::*;

    /// Bytes of the returned `ItemIcon` (the `egui_extras` loader only
    /// intervenes at drawing time: here we compare the raw source).
    fn icon_bytes(icon: &ItemIcon) -> &'static [u8] {
        match &icon.src {
            egui::ImageSource::Bytes {
                bytes: egui::load::Bytes::Static(b),
                ..
            } => b,
            other => panic!("unexpected source: {other:?}"),
        }
    }

    #[test]
    fn blueprint_sid_gets_the_blueprint_icon() {
        let bp = map()
            .get("_blueprint")
            .copied()
            .expect("_blueprint.png missing");
        for sid in ["Blueprint_D12_Upgrade_1", "blueprint_x", "X_BLuEprint"] {
            let icon = icon_for(sid);
            assert_eq!(
                icon_bytes(&icon),
                bp.0,
                "{sid} does not receive the blueprint icon"
            );
        }
    }

    #[test]
    fn non_blueprint_sid_keeps_the_placeholder() {
        let ph = map()
            .get("_placeholder")
            .copied()
            .expect("_placeholder.png missing");
        let icon = icon_for("UnknownItem_999");
        assert_eq!(icon_bytes(&icon), ph.0, "the placeholder should stay");
    }

    #[test]
    fn placeholder_flag_tracks_the_fallback() {
        assert!(
            icon_for("UnknownItem_999").is_placeholder,
            "unknown SID → placeholder"
        );
        assert!(
            !icon_for("GunPM_HG").is_placeholder,
            "a real icon is not a placeholder"
        );
        // The blueprint icon is a dedicated icon, not a placeholder.
        assert!(
            !icon_for("Blueprint_X").is_placeholder,
            "blueprint icon is a real icon"
        );
    }

    #[test]
    fn weapon_variants_resolve_the_base_weapon_icon() {
        let base = map().get("GunPM_HG").copied();
        let ph = map().get("_placeholder").copied();
        // Each variant resolves to an existing icon: the base weapon icon
        // (or its `Guard` variant, when it exists).
        for sid in ["GunPM_HG_Player", "GunPM_HG_NPC", "GuardGunPM_HG_NPC"] {
            let icon = icon_for(sid);
            let resolved = icon_bytes(&icon);
            let ok = base.is_some_and(|(b, ..)| b == resolved)
                || ph.is_some_and(|(b, ..)| b == resolved);
            assert!(
                ok,
                "{sid} resolves to neither the base icon nor the placeholder"
            );
        }
    }
}
