//! Custom AIDA64 widget geometry: the regions whose mechanism the captures pin exactly but whose
//! rendering egui 0.36 cannot express — the core has no Tree/TabBar, `SELECTED_CLASS` recolors
//! selected widget text while AIDA64 keeps `#FFFFFF`, and the scrollbar trough `#171917`? no:
//! `#171717` is *darker* than the panel, which egui's opacity model cannot produce.
//!
//! Evidence (read before changing anything here):
//! * `work/aida64/out/ksnip-20261014-003-ui-spec.md` §1 — tree row pitch 18 px, level indent 19 px,
//!   icon box 16 px, chevron slot 8 px, icon→label gap 2 px, selection stroke `#3C3C3C` over fill
//!   `#626262`, tight box (measured instance 81×18, ring left edge x=27 while the glyph crop starts
//!   x=29), scrollbar = 17 px column (`#171717` trough / `#4D4D4D` thumb / `#676767` arrow ink).
//! * §4 — tab row 19 px tall, measured widths [83,81,56,48,50,58], flat fill = panel bg; the
//!   *active* tab differs from the inactive ones only by a 1-px gap in the frame/separators.
//! * §2 — log-list header strip 24 px filled `#191919` over body `#202020`.
//! * `work/aida64/out/atlas-analysis.md` part C — the on-screen 16 px tree glyph is sourced from
//!   the `aida_icons10.dll` 16×16 32-bpp BGRA-alpha DIB family (capture crop x=29..44, MAD 8.64 vs
//!   runner-up 29.35), so the icon slot here *is* the asset slot; bitmaps are still unwired (crate
//!   open item 2).
//!
//! Contract (identical to `paint`): crisp geometry is painted as fill-rects only — no `Stroke`, no
//! rounded corners, nothing feathered. Unmeasured glyphs (chevron shape, icon bitmaps, arrow glyph)
//! reserve space and paint nothing: inventing an ink would violate the measurement contract.

use egui::{Align2, CornerRadius, FontFamily, FontId, Painter, Response, Sense, Ui, vec2};

use crate::metrics::{
    CHEVRON_SLOT, FONT_EM_PTS, HEADER_ROW_H, ICON_LABEL_GAP, SCROLLBAR_ARROW_ZONE,
    SELECTION_INSET_X, TAB_H, TREE_ICON, TREE_INDENT, TREE_ROW_H,
};
use crate::paint::{
    paint_box_1px, paint_hairline_v, paint_scrollbar_arrow_ink, paint_scrollbar_slot,
    paint_scrollbar_thumb, rect_of_edges, snap, snap_rect,
};
use crate::palette::{BG_HEADER_STRIP, BG_SELECTION, HAIRLINE, STROKE_SELECTION, TEXT_PRIMARY};

/// The crate's label font: em 12 px, `Proportional`, no TTF installed — the exact mapping
/// `style::aida64_style()` writes into `text_styles` (measured: cap height 8–9 px; font file open).
pub fn aida64_font() -> FontId {
    FontId {
        size: FONT_EM_PTS,
        family: FontFamily::Proportional,
    }
}

/// One painted tree row: which device rect holds what.
#[derive(Clone, Copy, Debug)]
pub struct TreeRowPaint {
    /// The allocated row rect; the 18 px pitch is enforced by the layout (`tree_row_rects`).
    pub row: egui::Rect,
    /// The tight selection box, `Some` only when `selected`: `#626262` fill + 1-px `#3C3C3C` ring.
    /// Its left edge sits `SELECTION_INSET_X` left of the icon — reproducing the measured instance
    /// (ring x=27..107 around an icon crop starting at x=29).
    pub box_rect: Option<egui::Rect>,
    /// Reserved 8 px chevron slot when `expanded` — painted count 0, the glyph is unmeasured.
    pub chevron_slot: Option<egui::Rect>,
    /// Reserved 16 px icon slot (the sourced `aida_icons10.dll` DIB box); 0 shapes until the atlas
    /// is wired in.
    pub icon_slot: egui::Rect,
    /// Where the label landed (vertically centered on the row, ink `#FFFFFF` always).
    pub text_rect: egui::Rect,
    /// Crisp fill-rects queued: 0 unselected, 5 selected (`paint_box_1px`).
    pub shapes: usize,
}

/// Paint one AIDA64 tree row inside `row`.
///
/// Selection is painted *behind* the label and the text stays `#FFFFFF`: egui's own selected-widget
/// path (`egui-0.36.2/src/style/widget_style.rs:150-155`) would recolor it to
/// `visuals.selection.stroke.color`, which the captures contradict.
pub fn paint_tree_row(
    p: &Painter,
    row: egui::Rect,
    label: &str,
    expanded: bool,
    selected: bool,
) -> TreeRowPaint {
    // Content order measured in spec §1: [chevron 8][icon 16][gap 2][label]. Selected and
    // unselected rows keep the *same* content x — only the box sticks out (measured left outset
    // 2 px; the right inset is assumed symmetric ⇒ unmeasured, see metrics note).
    let slot_x0 = row.left() + if expanded { CHEVRON_SLOT } else { 0.0 };
    let chevron_slot =
        expanded.then(|| rect_of_edges(row.left(), row.top(), slot_x0, row.bottom()));
    // Vertically centered 16-px glyph inside the 18-px band (1+1 pad rows): integer 1:1 texel
    // mapping at PPM 1 (the 16×16 asset is measured in `work/aida64/out/atlas/`); the horizontal
    // 16 px at the cursor is capture-measured (crop x=29..=44 = cursor 29 + 16), the exact
    // glyph-rect vertical correspondence in the capture needs window/2× captures ⇒ centered pads
    // are a layout choice (AGENTS rule 11: like the tile x-offsets), documented here.
    let icon_slot = rect_of_edges(
        slot_x0,
        row.top() + (TREE_ROW_H - TREE_ICON) / 2.0,
        slot_x0 + TREE_ICON,
        row.bottom() - (TREE_ROW_H - TREE_ICON) / 2.0,
    );

    let font = aida64_font();
    let text_rect = p.text(
        egui::pos2(slot_x0 + TREE_ICON + ICON_LABEL_GAP, row.center().y),
        Align2::LEFT_CENTER,
        label,
        font,
        TEXT_PRIMARY,
    );

    let box_rect = selected.then(|| {
        rect_of_edges(
            icon_slot.left() - SELECTION_INSET_X,
            row.top(),
            text_rect.right() + SELECTION_INSET_X,
            row.bottom(),
        )
    });
    let shapes = match box_rect {
        Some(b) => paint_box_1px(p, b, BG_SELECTION, STROKE_SELECTION),
        None => 0,
    };
    TreeRowPaint {
        row,
        box_rect,
        chevron_slot,
        icon_slot,
        text_rect,
        shapes,
    }
}

/// Row rects at the measured 18 px pitch (`ksnip_20261014-003601.png` rows: corr 0.794 @ pitch 18,
/// spec §1), snapped so every band edge lands on the device grid at PPM 1.
pub fn tree_row_rects(
    origin_left: f32,
    origin_top: f32,
    width: f32,
    rows: usize,
) -> Vec<egui::Rect> {
    (0..rows)
        .map(|i| {
            snap_rect(
                rect_of_edges(
                    origin_left,
                    origin_top + i as f32 * TREE_ROW_H,
                    origin_left + width,
                    origin_top + (i as f32 + 1.0) * TREE_ROW_H,
                ),
                1.0,
            )
        })
        .collect()
}

/// The level step is `TREE_INDENT`; egui `Ui::indent` advances by `spacing.indent`, which
/// `aida64_style()` sets to exactly that (spec §1: 19 px/level). This pins the relation so a
/// future spacing edit is caught by a test rather than silently shifting the tree.
pub fn indent_step_is_measured(spacing_indent: f32) -> bool {
    spacing_indent == TREE_INDENT
}

/// Paint the 1-px vertical separators between tabs, omitting every separator that touches the
/// *active* tab (spec §4: the active tab is distinguished only by a 1-px gap in the frame).
/// Returns the x-ids actually painted. Tabs are laid out contiguous at the measured widths —
/// inter-tab whitespace is unmeasured, so the separator *is* the shared 1-px edge (assumption
/// recorded here and in the crate docs).
pub fn paint_tab_separators(
    p: &Painter,
    strip_left: f32,
    widths: &[f32],
    y_top: f32,
    y_bottom: f32,
    active: Option<usize>,
) -> Vec<f32> {
    let ppm = 1.0; // capture calibration; callers at other PPM pass snapped widths already
    let mut painted = Vec::new();
    if widths.len() < 2 {
        return painted;
    }
    let mut x = strip_left;
    for (i, w) in widths.iter().take(widths.len() - 1).enumerate() {
        x += snap(*w, ppm);
        let gap_touches_active = active == Some(i) || active == Some(i + 1);
        if !gap_touches_active {
            paint_hairline_v(p, x, y_top, y_bottom, HAIRLINE);
            painted.push(x);
        }
    }
    painted
}

/// Interactive tab strip: allocates `TAB_H`-tall cells at the given widths (clickable via the
/// returned `Response`s) and paints the gap-only-active separator mechanism. Tab row height and
/// widths are measured (§4); tab *text* color per state is unmeasured ⇒ the crate-wide `#FFFFFF`
/// override applies.
pub fn tab_strip(
    ui: &mut Ui,
    labels: &[&str],
    widths: &[f32],
    active: Option<usize>,
) -> Vec<Response> {
    assert_eq!(
        labels.len(),
        widths.len(),
        "tab labels and widths must pair"
    );
    let band_top = ui.cursor().top();
    let strip_left = ui.cursor().left();
    let ppm = ui.pixels_per_point();
    let mut responses = Vec::with_capacity(widths.len());
    for w in widths {
        responses.push(ui.allocate_response(vec2(snap(*w, ppm), TAB_H), Sense::click()));
    }
    let _ = paint_tab_separators(
        ui.painter(),
        strip_left,
        widths,
        band_top,
        band_top + TAB_H,
        active,
    );
    responses
}

/// The measured 17-px scrollbar column, a thumb, and the two 17-px arrow zones. `arrows` toggles
/// only the `#676767` ink zones so a test can assert the exact delta.
pub fn paint_aida64_scrollbar(
    p: &Painter,
    slot: egui::Rect,
    thumb: egui::Rect,
    arrows: bool,
) -> usize {
    let mut n = paint_scrollbar_slot(p, slot);
    n += paint_scrollbar_thumb(p, thumb);
    if arrows {
        let top = rect_of_edges(
            slot.left(),
            slot.top(),
            slot.right(),
            slot.top() + SCROLLBAR_ARROW_ZONE,
        );
        let bottom = rect_of_edges(
            slot.left(),
            slot.bottom() - SCROLLBAR_ARROW_ZONE,
            slot.right(),
            slot.bottom(),
        );
        n += paint_scrollbar_arrow_ink(p, top);
        n += paint_scrollbar_arrow_ink(p, bottom);
    }
    n
}

/// Log-list header strip: 24 px of flat `#191919` over the `#202020` body (spec §2). The *body*
/// row pitch is unmeasured (the captured list was empty) ⇒ nothing below the strip is claimed.
pub fn paint_log_header_strip(p: &Painter, rect: egui::Rect) -> usize {
    let snapped = snap_rect(rect, p.pixels_per_point());
    debug_assert!(
        (snapped.height() - HEADER_ROW_H).abs() < 1e-3,
        "header strip height must be the measured 24 px"
    );
    let _ = p.rect_filled(snapped, CornerRadius::ZERO, BG_HEADER_STRIP);
    1 // exactly one fill rect: measured band, no stroke, no rounding
}

/// Runtime-loaded glyph bytes (straight RGBA — the atlas' native storage shape, see
/// `work/aida64/out/atlas-analysis.md`: 32-bpp BGRA DIBs decoded to straight alpha PNGs). This is
/// a data carrier, not a theme constant: AGENTS rule 10 keeps the atlas binaries out of the repo
/// (`work/` is gitignored), so the shell loads the provenance asset
/// `aida_icons10.dll_dib_16x16_32bpp_comp0_pal0_BGRA-alpha_92.png` (golden claim
/// `atlas-glyph-match`, MAD 1.02 over the selected-row crop) at runtime and hands its pixels here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IconPixels {
    pub width: usize,
    pub height: usize,
    pub rgba: Vec<u8>,
}

/// Paint measured glyph ink into a reserved icon slot. Nearest-neighbor texel mapping keeps the
/// asset's 1-px geometry crisp at PPM 1 (`TextureOptions::NEAREST`,
/// `epaint-0.36.2/src/textures.rs:191`); at PPM 2 callers must supply the 2× variants that
/// `aida_uireshd.dll` ships (measured in `work/aida64/out/atlas-inventory.txt`). Returns the count
/// of queued shapes (exactly 1 textured fill) so the raster gate can assert the delta.
pub fn paint_icon_rgba(
    ctx: &egui::Context,
    p: &Painter,
    slot: egui::Rect,
    glyph: &IconPixels,
) -> usize {
    debug_assert!(
        glyph.rgba.len() == glyph.width * glyph.height * 4,
        "IconPixels must be straight RGBA of exactly width*height*4 bytes"
    );
    // A uniquely named allocation per call is measured-necessary (`tests/gpu_snapshot.rs`:
    // stable-name glyph wiring rasterized zero texel pixels in the kittest GPU harness under
    // both wiring orders): whichever pass `render()` finally consumes then carries its own
    // add-delta and its own texture, so the glyph is uploaded before it is drawn in kittest AND
    // in eframe alike. 1 KB glyph churn per pass is negligible.
    static ICON_TEX_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let image = egui::ColorImage::from_rgba_unmultiplied([glyph.width, glyph.height], &glyph.rgba);
    let texture = ctx.load_texture(
        format!(
            "aida64_icon_{}x{}_{}_{}",
            glyph.width,
            glyph.height,
            glyph.rgba.len(),
            ICON_TEX_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ),
        image.clone(),
        egui::TextureOptions::NEAREST,
    );
    ICON_UPLOADS.with(|cell| cell.borrow_mut().insert(texture.id(), image));
    let _ = p.image(
        texture.id(),
        crate::paint::snap_rect(slot, p.pixels_per_point()),
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    1
}

thread_local! {
    // Measured in `tests/gpu_snapshot.rs`: kittest settle-passes drain texture deltas before the
    // offscreen read-pass runs, so neither the manager nor `FullOutput` still carries glyph
    // pixels at readback time (ui-test-58/62: empty `set` at both). The crate stashes every glyph
    // allocation here (id → full image) so the lane can upload it via `Renderer::update_texture`;
    // normal eframe apps never read this (their renderer consumes deltas as usual).
    static ICON_UPLOADS: std::cell::RefCell<
        std::collections::HashMap<egui::TextureId, egui::ColorImage>,
    > = std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Every glyph texture this crate allocated on the current thread, as `(id, full image)`. Used
/// by the offscreen GPU lane (`tests/gpu_snapshot.rs::render_scene_with_textures`); see the
/// `ICON_UPLOADS` note for why deltas are not enough there.
pub fn icon_texture_uploads() -> Vec<(egui::TextureId, egui::ColorImage)> {
    ICON_UPLOADS.with(|cell| {
        cell.borrow()
            .iter()
            .map(|(id, image)| (*id, image.clone()))
            .collect()
    })
}

/// A tree row as an `egui::Widget` so it composes in normal `ui.add(...)` layouts; callers handle
/// per-level indents with `Ui::indent` (which walks by the patched 19 px spacing). `icon_rgba`
/// carries runtime-loaded measured atlas bytes; when `None` the icon slot stays reserved with no
/// fabricated ink.
#[derive(Clone, Debug)]
pub struct TreeItem<'a> {
    pub label: &'a str,
    pub depth: usize,
    pub expanded: bool,
    pub selected: bool,
    pub icon_rgba: Option<IconPixels>,
}

impl egui::Widget for TreeItem<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let galley =
            ui.painter()
                .layout_no_wrap(self.label.to_string(), aida64_font(), TEXT_PRIMARY);
        let content_w = (if self.expanded { CHEVRON_SLOT } else { 0.0 })
            + TREE_ICON
            + ICON_LABEL_GAP
            + galley.size().x;
        let resp = ui.allocate_response(vec2(content_w, TREE_ROW_H), Sense::click());
        let painted = paint_tree_row(
            ui.painter(),
            resp.rect,
            self.label,
            self.expanded,
            self.selected,
        );
        if let Some(glyph) = self.icon_rgba.as_ref() {
            let _ = paint_icon_rgba(ui.ctx(), ui.painter(), painted.icon_slot, glyph);
        }
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::{SELECTION_BOX_H, TAB_WIDTHS_MEASURED};
    use egui::{Context, LayerId, Painter as EPainter, Rect, emath};

    fn painter_over(w: f32, h: f32) -> EPainter {
        // egui 0.36 installs its embedded default fonts only during the first pass of
        // `Context::run_ui` (measured: `egui-0.36.2/src/context.rs:1103,1121` — "No fonts available
        // until first call to Context::run()"). Prime the context with one empty pass before
        // painting text, or `Painter::text`/`layout_no_wrap` panic.
        let ctx = Context::default();
        let mut priming_output = ctx.run_ui(egui::RawInput::default(), |_ui| {});
        // The priming pass itself queues one texture delta; epaint panics when a `TexturesDelta`
        // with unapplied entries is dropped (`epaint-0.36.2/src/textures.rs:337`, measured here).
        priming_output.textures_delta.clear();
        EPainter::new(
            ctx,
            LayerId::background(),
            Rect::from_min_max(emath::pos2(0.0, 0.0), emath::pos2(w, h)),
        )
    }

    #[test]
    fn icon_glyph_paints_exactly_one_texture_shape() {
        let ctx = Context::default();
        let mut priming_output = ctx.run_ui(egui::RawInput::default(), |_ui| {});
        priming_output.textures_delta.clear(); // epaint textures.rs:337 drop-panic guard
        let p = EPainter::new(
            ctx.clone(),
            LayerId::background(),
            Rect::from_min_max(emath::pos2(0.0, 0.0), emath::pos2(300.0, 120.0)),
        );
        // 16-px slot at the measured tree-column position (panel left 8 + inner pad 19 ..
        // divider 201, spec §1); geometry asserted by the sibling tests, here only the ink delta.
        let slot = rect_of_edges(8.0, 8.0, 8.0 + TREE_ICON, 8.0 + TREE_ICON);
        let glyph = IconPixels {
            width: 16,
            height: 16,
            rgba: vec![255u8; 16 * 16 * 4],
        };
        let painted = paint_icon_rgba(&ctx, &p, slot, &glyph);
        assert_eq!(painted, 1, "one nearest-textured fill per loaded glyph");
    }

    #[test]
    fn tree_row_geometry_follows_the_measured_mechanism() {
        let p = painter_over(300.0, 120.0);
        // x = panel-left 8 + indent 19 .. divider 201 (spec §1); row height = TREE_ROW_H
        let row = rect_of_edges(27.0, 0.0, 201.0, TREE_ROW_H);
        let r = paint_tree_row(&p, row, "Computer", true, false);
        assert_eq!(r.shapes, 0); // unselected paints no crisp shape
        assert!(r.box_rect.is_none());
        let ch = r
            .chevron_slot
            .expect("expanded rows reserve the chevron slot");
        assert_eq!(ch.width(), CHEVRON_SLOT);
        assert_eq!(r.icon_slot.width(), TREE_ICON);
        assert_eq!(r.icon_slot.left(), row.left() + CHEVRON_SLOT);
        assert_eq!(r.text_rect.left() - r.icon_slot.right(), ICON_LABEL_GAP);
        assert!((r.text_rect.center().y - row.center().y).abs() < 1e-3);
        assert!(
            r.text_rect.width() > 0.0,
            "the label must actually be laid out"
        );
        // collapsed variant: content starts flush at the row's left edge
        let r2 = paint_tree_row(&p, row, "Settings", false, false);
        assert!(r2.chevron_slot.is_none());
        assert_eq!(r2.icon_slot.left(), row.left());
    }

    #[test]
    fn selection_box_reproduces_the_ring_mechanism_of_the_capture() {
        let p = painter_over(300.0, 120.0);
        let row = rect_of_edges(27.0, 0.0, 201.0, TREE_ROW_H);
        let r = paint_tree_row(&p, row, "Computer", false, true);
        let b = r.box_rect.expect("selected rows paint a box");
        assert_eq!(r.shapes, 5); // paint_box_1px: fill + 4 device-pixel edges
        assert_eq!(b.height(), SELECTION_BOX_H);
        assert_eq!(
            b.width(),
            TREE_ICON + ICON_LABEL_GAP + r.text_rect.width() + 2.0 * SELECTION_INSET_X
        );
        assert_eq!(b.left(), r.icon_slot.left() - SELECTION_INSET_X);
        assert_eq!(b.right(), r.text_rect.right() + SELECTION_INSET_X);
        assert_eq!(b.left().fract(), 0.0); // outset/gap are integer px ⇒ edges are crisp
        // The capture instance (ring x=27..107 around the icon at 29) is this same formula with a
        // 59 px ink run; egui's built-in font yields a different run, so only the mechanism is
        // asserted here — the 81 px instance stays in `metrics` + `tools/model_vs_capture.py`.
    }

    #[test]
    fn tree_rows_raster_at_the_measured_18px_pitch() {
        let rows = tree_row_rects(8.0, 0.0, 194.0, 11);
        assert_eq!(rows.len(), 11);
        for pair in rows.windows(2) {
            assert_eq!(pair[1].top() - pair[0].bottom(), 0.0); // contiguous bands
            assert_eq!(pair[1].top() - pair[0].top(), TREE_ROW_H); // measured pitch, no drift
            assert_eq!(pair[1].top().fract(), 0.0); // every band edge on the device grid
        }
    }

    #[test]
    fn tab_gap_is_the_only_active_tab_differentiator() {
        let p = painter_over(400.0, 24.0);
        let widths = TAB_WIDTHS_MEASURED;
        let all = paint_tab_separators(&p, 0.0, &widths, 0.0, TAB_H, None);
        assert_eq!(all.len(), widths.len() - 1); // five shared edges for six tabs
        let first_active = paint_tab_separators(&p, 0.0, &widths, 0.0, TAB_H, Some(0));
        assert_eq!(all.len() - first_active.len(), 1); // active at 0: only its right edge opens
        let mid_active = paint_tab_separators(&p, 0.0, &widths, 0.0, TAB_H, Some(3));
        assert_eq!(all.len() - mid_active.len(), 2); // an inner active tab opens both edges
        // painted x-ids equal the cumulative measured widths, snapped ⇒ exact integers at PPM 1
        let mut acc = 0.0;
        let mut expected = Vec::new();
        for w in &widths[..widths.len() - 1] {
            acc += *w;
            expected.push(acc);
        }
        assert_eq!(all, expected);
        for x in &all {
            assert_eq!(x.fract(), 0.0);
        }
    }

    #[test]
    fn scrollbar_composition_adds_exactly_the_two_arrow_zones() {
        let p = painter_over(20.0, 600.0);
        // Column geometry is the measured 17/15/1 relation; the x-position here is arbitrary
        // (the captures' absolute scrollbar x depends on the crop, spec §1 records the widths).
        let slot = rect_of_edges(0.0, 100.0, 17.0, 500.0);
        let thumb = rect_of_edges(0.0, 150.0, 15.0, 300.0);
        let bare = paint_aida64_scrollbar(&p, slot, thumb, false);
        let full = paint_aida64_scrollbar(&p, slot, thumb, true);
        assert_eq!(full - bare, 2); // paint_scrollbar_arrow_ink queues exactly 1 rect each
        assert!(full >= 4); // slot + thumb + 2 arrows, minimum
    }

    #[test]
    fn header_strip_is_24px_of_flat_fill_only() {
        let p = painter_over(600.0, 120.0);
        let rect = rect_of_edges(11.0, 63.0, 519.0, 87.0); // img4 x=11..519, 24 rows, spec §2
        assert_eq!(paint_log_header_strip(&p, rect), 1);
    }

    #[test]
    fn indent_spacing_is_the_measured_level_step() {
        assert!(indent_step_is_measured(
            crate::style::aida64_style().spacing.indent
        ));
    }
}
