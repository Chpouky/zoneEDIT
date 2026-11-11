//! zoneEDIT UI kit: theme + custom widgets reproducing the "A – Dark" mockup.
//! egui/eframe 0.31 (CornerRadius, StrokeKind, Frame::new).
//!
//! Needs: assets/fonts/Inter-Regular.ttf, assets/fonts/JetBrainsMono-Regular.ttf

use egui::{
    vec2, Context, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Frame, Margin,
    Response, Sense, Shadow, Stroke, StrokeKind, Ui,
};

pub mod c {
    use egui::Color32;
    pub const BG: Color32 = Color32::from_rgb(0x1A, 0x1A, 0x1A);
    pub const PANEL: Color32 = Color32::from_rgb(0x1F, 0x1F, 0x1F);
    pub const INPUT_BG: Color32 = Color32::from_rgb(0x14, 0x14, 0x14);
    pub const CONTROL_BG: Color32 = Color32::from_rgb(0x2A, 0x2A, 0x2A);
    pub const HOVER: Color32 = Color32::from_rgb(0x33, 0x33, 0x33);
    pub const BORDER: Color32 = Color32::from_rgb(0x33, 0x33, 0x33);
    pub const BORDER_CTRL: Color32 = Color32::from_rgb(0x3A, 0x3A, 0x3A);
    /// Outline of the row checkboxes (empty state): one step lighter
    /// than `BORDER_CTRL`, otherwise the box is lost on the dark background.
    pub const CB_BORDER: Color32 = Color32::from_rgb(0x4D, 0x4D, 0x4D);
    pub const ROW_LINE: Color32 = Color32::from_rgb(0x2B, 0x2B, 0x2B);
    pub const TEXT: Color32 = Color32::from_rgb(0xE5, 0xE5, 0xE5);
    pub const TEXT_DIM: Color32 = Color32::from_rgb(0xB4, 0xB4, 0xB4);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x8C, 0x8C, 0x8C);
    pub const TEXT_STRONG: Color32 = Color32::WHITE;
    pub const ACCENT: Color32 = Color32::from_rgb(0xF0, 0x8C, 0x2E);
    pub const ON_ACCENT: Color32 = Color32::from_rgb(0x14, 0x14, 0x14);
    pub const MODIFIED_FG: Color32 = Color32::from_rgb(0xE5, 0xC0, 0x7B);
    pub const MODIFIED_BG: Color32 = Color32::from_rgb(0x38, 0x33, 0x2A);
    pub const DANGER_FG: Color32 = Color32::from_rgb(0xEF, 0x7F, 0x88);
    pub const SUCCESS_FG: Color32 = Color32::from_rgb(0x8F, 0xC9, 0x6C);
    pub const DANGER_BG: Color32 = Color32::from_rgb(0x3A, 0x2B, 0x2B);
    /// Highlight tints of the delegations (delegation line + source block
    /// + target block share the same tint). Deliberately very light
    ///   (alpha ≈ 10 %): they must not steal the show from the data.
    pub const DELEG_TINTS: [Color32; 6] = [
        // Pre-multiplied values (α = 26/255 ≈ 10 %): blue, green, purple,
        // teal, pink, yellow — e.g. blue = 96,148,224,26 unmultiplied.
        Color32::from_rgba_premultiplied(10, 15, 23, 26), // bleu
        Color32::from_rgba_premultiplied(11, 20, 14, 26), // vert
        Color32::from_rgba_premultiplied(18, 14, 23, 26), // violet
        Color32::from_rgba_premultiplied(10, 20, 20, 26), // teal
        Color32::from_rgba_premultiplied(23, 15, 19, 26), // rose
        Color32::from_rgba_premultiplied(23, 20, 12, 26), // jaune
    ];
}

/// Call once at startup (in the eframe creation closure).
pub fn apply_theme(ctx: &Context) {
    // Fonts
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "inter".into(),
        std::sync::Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/Inter-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "jbmono".into(),
        std::sync::Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/JetBrainsMono-Regular.ttf"
        ))),
    );
    fonts
        .families
        .get_mut(&FontFamily::Proportional)
        .unwrap()
        .insert(0, "inter".into());
    fonts
        .families
        .get_mut(&FontFamily::Monospace)
        .unwrap()
        .insert(0, "jbmono".into());
    ctx.set_fonts(fonts);

    // Visuals
    let mut v = egui::Visuals::dark();
    v.override_text_color = Some(c::TEXT);
    v.panel_fill = c::PANEL;
    // The fork draws a vertical line to the left of each ui::indent:
    // disabled (it made a stray line in the sidebar).
    v.indent_has_left_vline = false;
    v.window_fill = c::PANEL;
    v.extreme_bg_color = c::INPUT_BG;
    v.faint_bg_color = c::PANEL;
    v.window_stroke = Stroke::new(1.0_f32, c::BORDER);
    v.window_shadow = Shadow::NONE;
    v.popup_shadow = Shadow::NONE;
    v.selection.bg_fill = c::ACCENT;
    v.selection.stroke = Stroke::new(1.0_f32, c::ON_ACCENT);

    let r = CornerRadius::same(4);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, c::BORDER);
    v.widgets.noninteractive.bg_fill = c::CONTROL_BG;
    v.widgets.noninteractive.weak_bg_fill = c::CONTROL_BG;
    for w in [
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
    ] {
        w.corner_radius = r;
        w.bg_stroke = Stroke::new(1.0_f32, c::BORDER_CTRL);
        // Gris discret (= `TEXT_DIM`) : les textes des widgets passent par
        // `override_text_color` (`TEXT`), so here it only touches the scrollbars'
        // handles (`foreground_color = true`) and the window glyphs
        // (X button of the modals) — a white (`TEXT`) was too aggressive.
        w.fg_stroke = Stroke::new(1.0_f32, c::TEXT_DIM);
    }
    v.widgets.inactive.bg_fill = c::CONTROL_BG;
    v.widgets.inactive.weak_bg_fill = c::CONTROL_BG;
    v.widgets.hovered.bg_fill = c::HOVER;
    v.widgets.hovered.weak_bg_fill = c::HOVER;
    v.widgets.active.bg_fill = c::HOVER;
    v.widgets.active.weak_bg_fill = c::HOVER;
    ctx.set_visuals(v);

    // Spacing / text sizes
    ctx.style_mut(|s| {
        s.spacing.item_spacing = vec2(8.0, 8.0);
        s.spacing.button_padding = vec2(12.0, 6.0);
        s.spacing.interact_size.y = 32.0;
        s.text_styles.insert(
            egui::TextStyle::Small,
            egui::FontId::proportional(crate::ui::metrics::FONT_SMALL),
        );
        s.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::proportional(crate::ui::metrics::FONT_BODY),
        );
        s.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::proportional(crate::ui::metrics::FONT_BUTTON),
        );
        s.text_styles
            .insert(egui::TextStyle::Heading, egui::FontId::proportional(15.0));
        // Scrollbars: thin, gray (`TEXT_DIM`), visible as soon as
        // the pointer is on the area (so during the wheel scroll).
        s.spacing.scroll = scroll_bar_style();
    });
}

/// Pill chip (radius 14, height 28). Active = orange.
pub fn chip(ui: &mut Ui, label: &str, active: bool) -> Response {
    let color = if active { c::ON_ACCENT } else { c::TEXT_DIM };
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), FontId::proportional(12.0), color);
    let size = vec2((galley.size().x + 20.0).max(44.0), 28.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let (fill, stroke) = if active {
        (c::ACCENT, c::ACCENT)
    } else if resp.hovered() {
        (c::HOVER, c::BORDER_CTRL)
    } else {
        (c::CONTROL_BG, c::BORDER_CTRL)
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(14),
        fill,
        Stroke::new(1.0_f32, stroke),
        StrokeKind::Inside,
    );
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, color);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Primary (orange) button, e.g. "Export (14)".
pub fn primary_button(ui: &mut Ui, label: &str, enabled: bool) -> Response {
    let text = egui::RichText::new(label).color(c::ON_ACCENT).strong();
    let b = egui::Button::new(text)
        .fill(c::ACCENT)
        .stroke(Stroke::new(1.0_f32, c::ACCENT))
        .min_size(vec2(44.0, 32.0));
    ui.add_enabled(enabled, b)
}

/// Small pill tag. `danger = false` -> yellow "refs" style, `true` -> red "orphan" style.
pub fn tag(ui: &mut Ui, text: &str, danger: bool) {
    let (fg, bg) = if danger {
        (c::DANGER_FG, c::DANGER_BG)
    } else {
        (c::MODIFIED_FG, c::MODIFIED_BG)
    };
    Frame::new()
        .fill(bg)
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).size(11.0).color(fg));
        });
}

/// Sidebar nav row: dim text + right-aligned count; selected = grey bg + 3px orange left bar.
pub fn nav_item(ui: &mut Ui, label: &str, count: usize, selected: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
    let p = ui.painter();
    if selected {
        p.rect_filled(rect, 0.0, c::BORDER);
        p.rect_filled(
            egui::Rect::from_min_size(rect.min, vec2(3.0, rect.height())),
            0.0,
            c::ACCENT,
        );
    } else if resp.hovered() {
        p.rect_filled(rect, 0.0, c::CONTROL_BG);
    }
    let txt = if selected {
        c::TEXT_STRONG
    } else {
        c::TEXT_DIM
    };
    p.text(
        rect.left_center() + vec2(25.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.0),
        txt,
    );
    p.text(
        rect.right_center() - vec2(12.0, 0.0),
        egui::Align2::RIGHT_CENTER,
        count.to_string(),
        FontId::monospace(11.0),
        c::TEXT_MUTED,
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// 1 px vertical separator in the border color, `h` tall, vertically centered
/// on the current layout line. Use it in bars (topbar, filter bar) instead of
/// painting raw 1 px rects.
pub fn vline(ui: &mut Ui, h: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(1.0, h), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, c::BORDER);
}

/// 1px horizontal separator in the border color (use between panels).
pub fn hline(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, c::BORDER);
}

/// Style of the app's scrollbars: `floating()` (thin, thicker
/// on hover of the handle), handle painted in `fg_stroke` (= `TEXT_DIM`, subtle
/// light gray) instead of white. `active_handle_opacity = 1.0`: the
/// handle is fully visible as soon as the pointer is on the
/// scroll area — so during the wheel scroll, not only on hover
/// of the bar. Applied globally in `apply_theme` via `spacing.scroll`
/// (egui 0.31 has no per-`ScrollArea` style).
fn scroll_bar_style() -> egui::style::ScrollStyle {
    let mut s = egui::style::ScrollStyle::floating();
    s.active_handle_opacity = 1.0;
    s
}
