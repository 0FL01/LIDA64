//! Crisp 1-device-pixel painting helpers for the parts of AIDA64 that an `egui` `Style` cannot
//! express (see `crate::style` module doc for the measured reasons).
//!
//! Rationale, measured: every border/hairline in the reference captures is exactly 1 pixel with
//! hard edges, while `egui` strokes are tessellated quads with feathering
//! (`egui-0.36.2/src/tessellator.rs`, `TessellationOptions::feathering_size_in_pixels`) and
//! `Stroke` carries no feather setting (`egui-0.36.2/src/stroke.rs:13-16` = `{ width, color }`).
//! Therefore hairlines here are painted as filled 1-device-px rectangles
//! (`Painter::rect_filled` with `CornerRadius::ZERO`), snapped to the device grid.
//!
//! Coordinate discipline: `egui` works in points; `Painter::pixels_per_point()` gives the device
//! scale. All helpers take/return points and convert through [`snap`], so at PPM 1.0 (the
//! calibration of the reference captures) they reproduce the measured pixel grid exactly, and at
//! PPM 2.0 they still land on device pixels instead of straddling two of them.
//!
//! Every paint helper returns the number of shapes it queued (`1` per hairline), which keeps a
//! pixel-diff harness able to reason about per-region shape budgets without reaching into egui's
//! private layer storage.

use egui::{Color32, CornerRadius, Painter, Rect, pos2};

use crate::metrics::*;
use crate::palette::*;

/// Convenience: a `Rect` from its four point-space edges `(x0, y0, x1, y1)`.
///
/// egui 0.36 `Rect` stores `min`/`max` `Pos2` corners (`emath-0.36.2/src/rect.rs:25-31`) and offers
/// `from_min_max` plus `left()/right()/top()/bottom()` accessors; this helper keeps the
/// measured-edge notation of the reference readable in the geometry code.
#[must_use]
pub fn rect_of_edges(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
    Rect::from_min_max(pos2(x0, y0), pos2(x1, y1))
}

/// Snap a point coordinate to the device-pixel grid of the given scale.
#[must_use]
pub fn snap(x: f32, ppm: f32) -> f32 {
    (x * ppm).round() / ppm
}

/// Width/height of exactly one device pixel, expressed in points.
#[must_use]
pub fn one_device_px(ppm: f32) -> f32 {
    1.0 / ppm
}

/// Snap every edge of a rect to the device grid.
#[must_use]
pub fn snap_rect(r: Rect, ppm: f32) -> Rect {
    rect_of_edges(
        snap(r.left(), ppm),
        snap(r.top(), ppm),
        snap(r.right(), ppm),
        snap(r.bottom(), ppm),
    )
}

/// The 1-device-px rectangle for a horizontal hairline at `y`, spanning x∈[x0,x1).
#[must_use]
pub fn hairline_rect_h(y: f32, x0: f32, x1: f32, ppm: f32) -> Rect {
    let y0 = snap(y, ppm);
    rect_of_edges(snap(x0, ppm), y0, snap(x1, ppm), y0 + one_device_px(ppm))
}

/// The 1-device-px rectangle for a vertical hairline at `x`, spanning y∈[y0,y1).
#[must_use]
pub fn hairline_rect_v(x: f32, y0: f32, y1: f32, ppm: f32) -> Rect {
    let x0 = snap(x, ppm);
    rect_of_edges(x0, snap(y0, ppm), x0 + one_device_px(ppm), snap(y1, ppm))
}

/// Paint a crisp horizontal hairline (the measured 1-px divider, e.g. the toolbar bottom line at
/// img1 y=81, whose color is the dither pair `#828790`/`#848494`).
pub fn paint_hairline_h(p: &Painter, y: f32, x0: f32, x1: f32, color: Color32) -> usize {
    let rect = hairline_rect_h(y, x0, x1, p.pixels_per_point());
    p.rect_filled(rect, CornerRadius::ZERO, color);
    1
}

/// Paint a crisp vertical hairline (e.g. the tree/content divider at img1 x=202).
pub fn paint_hairline_v(p: &Painter, x: f32, y0: f32, y1: f32, color: Color32) -> usize {
    let rect = hairline_rect_v(x, y0, y1, p.pixels_per_point());
    p.rect_filled(rect, CornerRadius::ZERO, color);
    1
}

/// Paint a horizontal hairline as the measured dither *pair* (`#828790` ↔ `#848494`), alternating
/// per device pixel; returns the number of pixels painted.
///
/// `unmeasured:` the alternation *order/period*. The reference row (img1 y=81) contains
/// `#828790` ×370 and `#848494` ×938 pixels, i.e. the pair is not a strict 50/50 checker, so this
/// is the honest interim model documented in spec §7 item 3 — a real ordered-dither LUT needs the
/// raw (non-re-encoded) window capture or the skin resource from the binary.
///
/// GPU-lane caveat measured here (`work/aida64/logs/ui-test-24.out` + `ui-test-25.out`): a filled
/// rect that is one device pixel in *both* dimensions tessellates to a degenerate (zero-width) core
/// and rasterizes to 0 px under `RendererOptions::PREDICTABLE` — so this helper's 20 alternating
/// cells paint nothing through `egui-wgpu` even though their geometry is exact. The GPU test lane
/// asserts that measured behavior (`tests/gpu_snapshot.rs::tessellation_probe_of_tiny_and_long_
/// device_rects`); a real GPU build needs a UV-textured quad or a shader for the dither.
pub fn paint_dither_pair_h(p: &Painter, y: f32, x0: f32, x1: f32, a: Color32, b: Color32) -> usize {
    let ppm = p.pixels_per_point();
    let y_top = snap(y, ppm);
    let y_bot = y_top + one_device_px(ppm);
    let mut x = snap(x0, ppm);
    let xe = snap(x1, ppm);
    let mut n = 0usize;
    while x < xe {
        let color = if n.is_multiple_of(2) { a } else { b };
        p.rect_filled(
            rect_of_edges(x, y_top, x + one_device_px(ppm), y_bot),
            CornerRadius::ZERO,
            color,
        );
        x += one_device_px(ppm);
        n += 1;
    }
    n
}

/// Fill `rect` and draw its 1-px outline *inside* the rect (StrokeKind::Inside equivalent): the
/// outline occupies the outer device-px ring, the fill covers `rect` minus that ring.
///
/// This is the shape of the measured tree selection (outer 81 × 18 px = fill 79 × 16 + 1-px ring,
/// `metrics::SELECTION_BOX_*`); returns 4 (+1 when a non-empty interior exists).
pub fn paint_box_1px(p: &Painter, rect: Rect, fill: Color32, outline: Color32) -> usize {
    let ppm = p.pixels_per_point();
    let inset = one_device_px(ppm);
    let (x0, y0, x1, y1) = (rect.left(), rect.top(), rect.right(), rect.bottom());
    let mut n = 0usize;
    n += paint_hairline_h(p, y0, x0, x1, outline);
    n += paint_hairline_h(p, y1 - inset, x0, x1, outline);
    n += paint_hairline_v(p, x0, y0, y1, outline);
    n += paint_hairline_v(p, x1 - inset, y0, y1, outline);
    let inner = snap_rect(
        rect_of_edges(x0 + inset, y0 + inset, x1 - inset, y1 - inset),
        ppm,
    );
    if inner.width() > 0.0 && inner.height() > 0.0 {
        p.rect_filled(inner, CornerRadius::ZERO, fill);
        n += 1;
    }
    n
}

/// The measured tree selection box (`#626262` fill + 1-px `#3C3C3C` ring, hard edges).
pub fn paint_selection_box(p: &Painter, outer: Rect) -> usize {
    paint_box_1px(p, outer, BG_SELECTION, STROKE_SELECTION)
}

/// Paint the four outer edges of `rect` in four different colors, each 1 device px.
///
/// Needed because both measured frames are asymmetric:
/// * img4 dialog: top `#313131`, right `#1A1A1A`, left `#202020`, bottom `#202020`.
/// * img1 search field: left `#FFFFFF`, top/right/bottom `#7A7A7A`.
pub fn paint_edges4(
    p: &Painter,
    rect: Rect,
    top: Color32,
    right: Color32,
    bottom: Color32,
    left: Color32,
) -> usize {
    let inset = one_device_px(p.pixels_per_point());
    let mut n = 0usize;
    n += paint_hairline_h(p, rect.top(), rect.left(), rect.right(), top);
    n += paint_hairline_h(p, rect.bottom() - inset, rect.left(), rect.right(), bottom);
    n += paint_hairline_v(p, rect.left(), rect.top(), rect.bottom(), left);
    n += paint_hairline_v(p, rect.right() - inset, rect.top(), rect.bottom(), right);
    n
}

/// The measured img4 dialog outer frame.
pub fn paint_dialog_edges(p: &Painter, rect: Rect) -> usize {
    paint_edges4(
        p,
        rect,
        STROKE_FRAME_TOP,
        STROKE_FRAME_RIGHT,
        BG_PANEL,
        BG_PANEL,
    )
}

/// The measured img1 search-field frame (interior is filled by the widget; only the frame is
/// painted here). Size reference: [`metrics::SEARCH_FIELD_W`] × [`metrics::SEARCH_FIELD_H`].
pub fn paint_field_frame(p: &Painter, rect: Rect) -> usize {
    paint_edges4(
        p,
        rect,
        STROKE_FIELD_OTHER,
        STROKE_FIELD_OTHER,
        STROKE_FIELD_OTHER,
        STROKE_FIELD_LEFT,
    )
}

/// A graph panel: black background plus the measured 1-px green grid; returns the shape count
/// (1 fill + interior columns + interior rows).
///
/// `row_pitch` is per panel ([`metrics::GRAPH_ROW_PITCH_1`] for the 150-px panel,
/// [`metrics::GRAPH_ROW_PITCH_2`] for the 146-px panel); the column pitch is always
/// [`metrics::GRAPH_COL_PITCH`] (measured exactly 10.0 px on both panels). Only interior lines are
/// painted — the panel border, if any, is drawn by the caller.
///
/// `unmeasured:` axis-label glyph colors (spec §7 item 5), so no labels are painted here.
pub fn paint_graph_panel(p: &Painter, rect: Rect, row_pitch: f32) -> usize {
    let ppm = p.pixels_per_point();
    let r = snap_rect(rect, ppm);
    p.rect_filled(r, CornerRadius::ZERO, BG_VALUE_FIELD);
    let mut n = 1usize;
    let mut x = r.left() + GRAPH_COL_PITCH;
    while x < r.right() - one_device_px(ppm) {
        n += paint_hairline_v(p, x, r.top(), r.bottom(), GRID_GRAPH);
        x += GRAPH_COL_PITCH;
    }
    let mut y = r.top() + row_pitch;
    while y < r.bottom() - one_device_px(ppm) {
        n += paint_hairline_h(p, y, r.left(), r.right(), GRID_GRAPH);
        y += row_pitch;
    }
    n
}

/// The plotted trace color ([`palette::TRACE_GRAPH`] `#FFFF00`), exposed so a graph widget uses the
/// measured value instead of a literal; the polyline geometry itself is app data.
#[must_use]
pub fn trace_color() -> Color32 {
    TRACE_GRAPH
}

/// Scrollbar slot: measured 17 px wide (`metrics::SCROLLBAR_TOTAL_W`), trough `#171717`, whose two
/// outer columns are the same color — so painting the whole slot as trough reproduces the measured
/// pixels exactly. The trough being *darker* than the panel is why egui's opacity model is unused
/// (`crate::style` module doc, item 3).
pub fn paint_scrollbar_slot(p: &Painter, slot: Rect) -> usize {
    p.rect_filled(
        snap_rect(slot, p.pixels_per_point()),
        CornerRadius::ZERO,
        BG_SCROLLBAR_TROUGH,
    );
    1
}

/// Thumb inside the slot: `#4D4D4D`. The measured thumb sits inside the 15-px body
/// ([`metrics::SCROLLBAR_BODY_W`]); the caller computes its rect from scroll state.
pub fn paint_scrollbar_thumb(p: &Painter, thumb: Rect) -> usize {
    p.rect_filled(
        snap_rect(thumb, p.pixels_per_point()),
        CornerRadius::ZERO,
        BG_SCROLLBAR_THUMB,
    );
    1
}

/// Arrow glyph band: `unmeasured:` shape of the glyph. Only its 3-px ink height
/// ([`metrics::SCROLLBAR_ARROW_GLYPH`]) and color `#676767` are measured, so this paints that
/// measured ink extent as a horizontal band centered in a 17-px arrow zone
/// ([`metrics::SCROLLBAR_ARROW_ZONE`]) and leaves the classic triangle to whoever measures it next.
pub fn paint_scrollbar_arrow_ink(p: &Painter, zone: Rect) -> usize {
    let ppm = p.pixels_per_point();
    let z = snap_rect(zone, ppm);
    let band = SCROLLBAR_ARROW_GLYPH * one_device_px(ppm);
    let cy = (z.top() + z.bottom()) * 0.5;
    p.rect_filled(
        rect_of_edges(
            z.left() + one_device_px(ppm),
            snap(cy - band / 2.0, ppm),
            z.right() - one_device_px(ppm),
            snap(cy + band / 2.0, ppm),
        ),
        CornerRadius::ZERO,
        GLYPH_SCROLLBAR_ARROW,
    );
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Context, LayerId};

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn snapping_lands_on_the_device_grid() {
        assert_eq!(snap(12.0, 1.0), 12.0);
        assert_eq!(snap(12.3, 2.0), 12.5); // 24.6 device px → 25 → 12.5 points
        assert_eq!(snap(12.5, 2.0), 12.5); // already aligned
        assert_eq!(one_device_px(1.0), 1.0);
        assert_eq!(one_device_px(2.0), 0.5);
        assert_eq!(one_device_px(4.0), 0.25);
    }

    #[test]
    fn hairlines_are_exactly_one_device_pixel_thick() {
        for ppm in [1.0f32, 2.0, 3.0] {
            let h = hairline_rect_h(10.4, 3.2, 100.7, ppm);
            let v = hairline_rect_v(10.4, 3.2, 100.7, ppm);
            assert!(
                close(h.height(), one_device_px(ppm)),
                "h hairline ppm {ppm}"
            );
            assert!(close(v.width(), one_device_px(ppm)), "v hairline ppm {ppm}");
            for coord in [
                h.top(),
                h.bottom(),
                h.left(),
                h.right(),
                v.left(),
                v.right(),
                v.top(),
                v.bottom(),
            ] {
                let dev = coord * ppm;
                assert!(
                    (dev - dev.round()).abs() < 1e-4,
                    "non-integer device coord {coord}"
                );
            }
        }
        // At the calibration scale (PPM 1.0) an integer y produces the measured grid exactly:
        // img1 row 81 between x=202..925 is a single scanline.
        let h = hairline_rect_h(81.0, 202.0, 925.0, 1.0);
        assert_eq!(h, rect_of_edges(202.0, 81.0, 925.0, 82.0));
    }

    #[test]
    fn box_1px_produces_the_measured_selection_geometry() {
        // Reference: outer 81 × 18 px, interior #626262 = 79 × 16, ring #3C3C3C (spec §5).
        let outer = rect_of_edges(
            27.0,
            90.0,
            27.0 + SELECTION_BOX_W_MEASURED,
            90.0 + SELECTION_BOX_H,
        );
        assert_eq!(outer.width(), 81.0);
        assert_eq!(outer.height(), 18.0);
        let inset = one_device_px(CALIBRATION_PPM);
        let inner = rect_of_edges(
            outer.left() + inset,
            outer.top() + inset,
            outer.right() - inset,
            outer.bottom() - inset,
        );
        assert_eq!(inner.width(), 79.0);
        assert_eq!(inner.height(), 16.0);
    }

    #[test]
    fn painting_executes_against_egui_and_reports_shape_budgets() {
        let ctx = Context::default();
        let p = Painter::new(
            ctx,
            LayerId::background(),
            rect_of_edges(0.0, 0.0, 400.0, 400.0),
        );

        // Asymmetric frames: exactly one 1-px rect per edge, no fills.
        assert_eq!(
            paint_dialog_edges(&p, rect_of_edges(6.0, 2.0, 770.0, 590.0)),
            4
        );
        assert_eq!(
            paint_field_frame(
                &p,
                rect_of_edges(10.0, 40.0, 10.0 + SEARCH_FIELD_W, 40.0 + SEARCH_FIELD_H)
            ),
            4
        );
        // Selection box: 4 ring edges + 1 interior fill.
        assert_eq!(
            paint_selection_box(
                &p,
                rect_of_edges(
                    27.0,
                    90.0,
                    27.0 + SELECTION_BOX_W_MEASURED,
                    90.0 + SELECTION_BOX_H
                )
            ),
            5
        );
        // Dither pair: one shape per device pixel between 202 and 206.
        assert_eq!(
            paint_dither_pair_h(&p, 81.0, 202.0, 206.0, HAIRLINE, HAIRLINE_ALT),
            4
        );
        // Scrollbar: 17-px slot, 15-px thumb, arrow ink — all execute and budget 1 shape each.
        let slot = rect_of_edges(190.0, 90.0, 190.0 + SCROLLBAR_TOTAL_W, 300.0);
        assert_eq!(slot.width(), SCROLLBAR_TOTAL_W);
        assert_eq!(paint_scrollbar_slot(&p, slot), 1);
        let thumb = rect_of_edges(
            slot.left() + SCROLLBAR_EDGE_W,
            slot.top(),
            slot.left() + SCROLLBAR_EDGE_W + SCROLLBAR_BODY_W,
            slot.top() + 40.0,
        );
        assert_eq!(thumb.width(), SCROLLBAR_BODY_W);
        assert_eq!(paint_scrollbar_thumb(&p, thumb), 1);
        let arrow_zone = rect_of_edges(
            slot.left(),
            slot.top(),
            slot.left() + SCROLLBAR_TOTAL_W,
            slot.top() + SCROLLBAR_ARROW_ZONE,
        );
        assert_eq!(paint_scrollbar_arrow_ink(&p, arrow_zone), 1);
    }

    #[test]
    fn graph_panel_uses_the_measured_pitches() {
        // Panel 1: 150 px tall, 9-px interior row pitch, 10-px column pitch (measured exactly).
        let rect = rect_of_edges(73.0, 300.0, 73.0 + 101.0, 300.0 + GRAPH_PANEL_H_1);
        let ctx = Context::default();
        let p = Painter::new(
            ctx,
            LayerId::background(),
            rect_of_edges(0.0, 0.0, 900.0, 900.0),
        );
        // Interior lines sit at top + m*pitch (m ≥ 1) while strictly below bottom − 1 px, i.e.
        // m < (size − 1)/pitch  ⇒  count = ceil((size−1)/pitch) − 1.
        let cols = ((rect.width() - 1.0) / GRAPH_COL_PITCH).ceil() as usize - 1;
        let rows = ((rect.height() - 1.0) / GRAPH_ROW_PITCH_1).ceil() as usize - 1;
        assert_eq!(
            paint_graph_panel(&p, rect, GRAPH_ROW_PITCH_1),
            1 + cols + rows,
            "fill + interior columns + interior rows"
        );
        // A panel narrower/taller than one pitch paints the black fill only.
        let ctx2 = Context::default();
        let p2 = Painter::new(
            ctx2,
            LayerId::background(),
            rect_of_edges(0.0, 0.0, 50.0, 50.0),
        );
        assert_eq!(
            paint_graph_panel(&p2, rect_of_edges(0.0, 0.0, 9.0, 7.0), GRAPH_ROW_PITCH_2),
            1
        );
    }

    #[test]
    fn graph_pitches_are_the_measured_ones() {
        assert_eq!(GRAPH_COL_PITCH, 10.0);
        assert_eq!(GRAPH_PANEL_H_1, 150.0);
        assert_eq!(GRAPH_ROW_PITCH_1, 9.0);
        assert_eq!(GRAPH_PANEL_H_2, 146.0);
        assert_eq!(GRAPH_ROW_PITCH_2, 8.0);
        assert_eq!(GRAPH_PANEL_GUTTER, 4.0);
        assert_eq!(GRAPH_AXIS_GUTTER_W, 56.0);
        assert_eq!(GRAPH_RIGHT_GUTTER_W, 89.0);
        assert_eq!(trace_color(), TRACE_GRAPH);
    }
}
