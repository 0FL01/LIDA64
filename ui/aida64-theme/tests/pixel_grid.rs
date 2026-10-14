//! CPU raster gate for the AIDA64 paint layer — no GPU, no extra crates.
//!
//! The theme's crispness contract (spec §6.6) is that every measured border/hairline/box is
//! *exactly one device pixel* sitting on the device-pixel grid, with the measured color, and that
//! region heights rasterize to the measured row counts. egui cannot be trusted on that point
//! because its strokes are feathered quads, so this harness does the honest thing: it asks egui
//! for the shapes it actually queued (`Painter::for_each_shape`, or `FullOutput.shapes` after
//! `Context::run_ui`), then rasterizes those axis-aligned rect fills into a CPU canvas and probes
//! the pixels.
//!
//! Two things are therefore proven here, not merely asserted in style-level unit tests:
//! 1. every queued shape is a feather-free, blur-free, radius-free, untextured rect whose edges
//!    land on integer device pixels (at PPM 1.0 *and* 2.0/3.0), and
//! 2. the painted pixels match the measured hexes, pitches and thicknesses pixel for pixel.
//!
//! The captures' calibration is PPM 1.0 (`crate::metrics::CALIBRATION_PPM`); the PPM 2/3 runs
//! prove the snapping, not the reference colors.

use aida64_ui_theme::metrics::{
    GRAPH_COL_PITCH, GRAPH_ROW_PITCH_1, GRAPH_ROW_PITCH_2, HEADER_ROW_H, SCROLLBAR_ARROW_GLYPH,
    SCROLLBAR_ARROW_ZONE, SCROLLBAR_BODY_W, SCROLLBAR_TOTAL_W, SEARCH_FIELD_H, SEARCH_FIELD_W,
    SELECTION_BOX_H, SELECTION_BOX_W_MEASURED, TITLEBAR_H, TOOLBAR_H,
};
use aida64_ui_theme::paint::{
    paint_dialog_edges, paint_dither_pair_h, paint_field_frame, paint_graph_panel,
    paint_hairline_h, paint_hairline_v, paint_scrollbar_arrow_ink, paint_scrollbar_slot,
    paint_scrollbar_thumb, paint_selection_box, rect_of_edges, trace_color,
};
use aida64_ui_theme::palette::{
    BG_HEADER_STRIP, BG_PANEL, BG_SCROLLBAR_THUMB, BG_SCROLLBAR_TROUGH, BG_SELECTION, BG_TITLEBAR,
    BG_VALUE_FIELD, GLYPH_SCROLLBAR_ARROW, GRID_GRAPH, HAIRLINE, HAIRLINE_ALT, STROKE_FIELD_LEFT,
    STROKE_FIELD_OTHER, STROKE_FRAME_RIGHT, STROKE_FRAME_TOP, STROKE_SELECTION, TRACE_GRAPH,
};
use aida64_ui_theme::{header_strip_frame, titlebar_frame, toolbar_frame};
use egui::{
    Color32, Context, CornerRadius, LayerId, Painter, RawInput, Rect, Shape, Stroke, Ui,
    ViewportId, ViewportInfo,
};

// ── Harness plumbing ─────────────────────────────────────────────────────────

/// What egui actually queued, reduced to the only shape kind the paint layer may use.
#[derive(Debug, Clone, Copy)]
struct RectRecord {
    rect: Rect,
    clip: Rect,
    fill: Color32,
    stroke: Stroke,
    radius: CornerRadius,
    blur: f32,
    angle: f32,
    textured: bool,
}

fn push_record(out: &mut Vec<RectRecord>, clip: Rect, shape: &Shape) {
    let Shape::Rect(rs) = shape else {
        panic!(
            "the AIDA64 paint layer must queue rect fills only (crisp 1-px contract), got {shape:?}"
        );
    };
    out.push(RectRecord {
        rect: rs.rect,
        clip,
        fill: rs.fill,
        stroke: rs.stroke,
        radius: rs.corner_radius,
        blur: rs.blur_width,
        angle: rs.angle,
        textured: rs.brush.is_some(),
    });
}

/// Drain the shapes of a [`Painter`]'s layer. `for_each_shape` takes the reader by value
/// (`egui-0.36.2/src/painter.rs:252`), so a moved closure type-checks where `&mut |s| …` hits the
/// HRTB wall — that is why the paint helpers also report shape budgets themselves.
fn painter_records(p: &Painter) -> Vec<RectRecord> {
    let mut out: Vec<RectRecord> = Vec::new();
    p.for_each_shape(|s| push_record(&mut out, s.clip_rect, &s.shape));
    out
}

/// An `f32` that sits on the device grid within float noise.
fn on_grid(v_dev: f32, tag: &str, edge: &str) {
    assert!(
        (v_dev - v_dev.round()).abs() < 1e-4,
        "{tag}: edge {edge} = {v_dev} device px is not on the device grid"
    );
}

fn dev_edges(r: Rect, ppm: f32, tag: &str) -> [i64; 4] {
    let (l, t, rt, b) = (
        r.left() * ppm,
        r.top() * ppm,
        r.right() * ppm,
        r.bottom() * ppm,
    );
    on_grid(l, tag, "left");
    on_grid(t, tag, "top");
    on_grid(rt, tag, "right");
    on_grid(b, tag, "bottom");
    [
        l.round() as i64,
        t.round() as i64,
        rt.round() as i64,
        b.round() as i64,
    ]
}

/// Clip rects come from egui layout and are not part of the crispness contract, so they are
/// rounded without the alignment assertion.
fn dev_span(r: Rect, ppm: f32) -> [i64; 4] {
    [
        (r.left() * ppm).round() as i64,
        (r.top() * ppm).round() as i64,
        (r.right() * ppm).round() as i64,
        (r.bottom() * ppm).round() as i64,
    ]
}

/// A canvas of device pixels, with the probes the measured facts need.
struct Canvas {
    w: usize,
    h: usize,
    ppm: f32,
    px: Vec<Color32>,
}

impl Canvas {
    fn new(w: usize, h: usize, ppm: f32) -> Self {
        Self {
            w,
            h,
            ppm,
            px: vec![Color32::TRANSPARENT; w * h],
        }
    }

    /// Paint one record: rect ∩ clip, in device pixels. Rects are device-aligned by contract, so
    /// a fill maps to whole rows/columns — never a partial pixel, never a feathered edge.
    fn paint(&mut self, rec: &RectRecord) {
        let tag = format!("rect {} @ {}", hex(rec.fill), rec.rect);
        let [mut x0, mut y0, mut x1, mut y1] = dev_edges(rec.rect, self.ppm, &tag);
        let [cx0, cy0, cx1, cy1] = dev_span(rec.clip, self.ppm);
        x0 = x0.max(cx0).max(0);
        y0 = y0.max(cy0).max(0);
        x1 = x1.min(cx1).min(self.w as i64);
        y1 = y1.min(cy1).min(self.h as i64);
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        let (x0, x1) = (x0 as usize, x1 as usize);
        let (y0, y1) = (y0 as usize, y1 as usize);
        let rows = y1 - y0;
        let start = y0 * self.w;
        for row in self.px[start..start + rows * self.w].chunks_exact_mut(self.w) {
            row[x0..x1].fill(rec.fill);
        }
    }

    fn paint_all(&mut self, recs: &[RectRecord]) {
        for rec in recs {
            self.paint(rec);
        }
    }

    fn px(&self, x: usize, y: usize) -> Color32 {
        self.px[y * self.w + x]
    }

    fn count(&self, c: Color32) -> usize {
        self.px.iter().filter(|p| **p == c).count()
    }

    fn painted_total(&self) -> usize {
        self.px
            .iter()
            .filter(|p| **p != Color32::TRANSPARENT)
            .count()
    }

    /// Rows that contain at least one pixel of `c`, top to bottom.
    fn rows_of(&self, c: Color32) -> Vec<usize> {
        (0..self.h)
            .filter(|&y| (0..self.w).any(|x| self.px(x, y) == c))
            .collect()
    }

    /// Columns that contain at least one pixel of `c`, left to right.
    fn cols_of(&self, c: Color32) -> Vec<usize> {
        (0..self.w)
            .filter(|&x| (0..self.h).any(|y| self.px(x, y) == c))
            .collect()
    }

    fn rows_spanning_full_width(&self, c: Color32) -> Vec<usize> {
        (0..self.h)
            .filter(|&y| (0..self.w).all(|x| self.px(x, y) == c))
            .collect()
    }

    /// Rows that contain **exactly** `n` pixels of `c`.
    ///
    /// Needed because `rows_of` marks a row even when a single line crosses it: a full-width
    /// hairline row has count `w`, while a row merely crossed by a few vertical lines has a small
    /// count. Same for [`Canvas::cols_with_count`].
    fn rows_with_count(&self, c: Color32, n: usize) -> Vec<usize> {
        (0..self.h)
            .filter(|&y| (0..self.w).filter(|&x| self.px(x, y) == c).count() == n)
            .collect()
    }

    fn cols_with_count(&self, c: Color32, n: usize) -> Vec<usize> {
        (0..self.w)
            .filter(|&x| (0..self.h).filter(|&y| self.px(x, y) == c).count() == n)
            .collect()
    }
}

fn canvas_of(w: usize, h: usize, recs: &[RectRecord], ppm: f32) -> Canvas {
    let mut canvas = Canvas::new(w, h, ppm);
    canvas.paint_all(recs);
    canvas
}

/// egui input for a canvas of `w`×`h` device pixels at `ppm`.
///
/// Measured API fact (`egui-0.36.2/src/context.rs:451-455`): the device scale of a pass is
/// `zoom_factor × native_pixels_per_point`, and `native_pixels_per_point` arrives only through
/// `ViewportInfo`. `Context::set_pixels_per_point` is a *zoom* adjustment that only becomes active
/// on the next pass (`context.rs:2296-2300`, `2337-2348`), so setting both multiplies the scale —
/// measured here: requesting PPM 2.0 that way yielded 4.0. The harness therefore feeds the scale
/// as input and never touches the zoom factor.
fn input_of(w: usize, h: usize, ppm: f32) -> RawInput {
    let area = rect_of_edges(0.0, 0.0, w as f32 / ppm, h as f32 / ppm);
    let mut input = RawInput {
        screen_rect: Some(area),
        focused: true,
        ..Default::default()
    };
    input.viewports.insert(
        ViewportId::ROOT,
        ViewportInfo {
            native_pixels_per_point: Some(ppm),
            ..Default::default()
        },
    );
    input
}

/// A context whose device scale is live. A [`Painter`] reads `ctx.pixels_per_point()` at
/// construction time (`egui-0.36.2/src/painter.rs:47-57`) and the native scale only becomes live
/// after a pass, hence this priming pass.
fn primed_context(w: usize, h: usize, ppm: f32) -> Context {
    let ctx = Context::default();
    let mut out = ctx.run_ui(input_of(w, h, ppm), |_ui| {});
    // `TexturesDelta::drop` panics on unapplied deltas (measured here: one font-atlas delta), so
    // the harness consumes it explicitly; a rect-only canvas needs no textures.
    out.textures_delta.clear();
    assert_eq!(out.pixels_per_point, ppm, "priming pass scale drift");
    assert!(
        out.shapes.is_empty(),
        "an empty priming pass must queue nothing"
    );
    assert_eq!(ctx.pixels_per_point(), ppm, "live scale drift");
    ctx
}

/// Paint through a real egui [`Painter`] (background layer), then raster what egui queued.
fn painter_scene(
    w: usize,
    h: usize,
    ppm: f32,
    f: impl FnOnce(&Painter) -> usize,
) -> (usize, Canvas, Vec<RectRecord>) {
    let ctx = primed_context(w, h, ppm);
    let area = rect_of_edges(0.0, 0.0, w as f32 / ppm, h as f32 / ppm);
    let painter = Painter::new(ctx.clone(), LayerId::background(), area);
    assert_eq!(painter.pixels_per_point(), ppm, "painter scale drift");
    let queued = f(&painter);
    let recs = painter_records(&painter);
    assert_eq!(
        queued,
        recs.len(),
        "a paint helper's reported shape budget must equal what egui queued"
    );
    (queued, canvas_of(w, h, &recs, ppm), recs)
}

/// Paint through a real egui layout pass (`run_ui`), e.g. a panel with an AIDA64 frame.
fn ui_scene(
    w: usize,
    h: usize,
    ppm: f32,
    mut f: impl FnMut(&mut Ui) -> usize,
) -> (usize, Canvas, Vec<RectRecord>) {
    let ctx = Context::default();
    // `run_ui` wants `FnMut(&mut Ui)` (unit return), so the region's own shape count is dropped:
    // egui can run several layout passes per call, which is why these tests probe final pixels
    // rather than shape counts.
    let mut out = ctx.run_ui(input_of(w, h, ppm), |ui| {
        let _ = f(ui);
    });
    out.textures_delta.clear();
    assert_eq!(out.pixels_per_point, ppm, "harness device scale drift");
    let mut recs: Vec<RectRecord> = Vec::new();
    for s in &out.shapes {
        push_record(&mut recs, s.clip_rect, &s.shape);
    }
    (0, canvas_of(w, h, &recs, ppm), recs)
}

/// The crispness invariants, checked on every single queued rect at a given scale.
fn assert_crisp(rects: &[RectRecord], ppm: f32) {
    for (i, r) in rects.iter().enumerate() {
        let tag = format!("shape #{i} @ ppm {ppm}");
        assert_ne!(r.fill, Color32::TRANSPARENT, "{tag}: nothing to paint");
        assert_eq!(r.stroke, Stroke::NONE, "{tag}: feathered stroke leaked in");
        assert_eq!(
            r.radius,
            CornerRadius::ZERO,
            "{tag}: rounded corner leaked in"
        );
        assert_eq!(r.blur, 0.0, "{tag}: blur (shadow/glow) leaked in");
        assert_eq!(r.angle, 0.0, "{tag}: rotated rect is not pixel-aligned");
        assert!(!r.textured, "{tag}: textured rect is not a flat fill");
        dev_edges(r.rect, ppm, &tag);
    }
}

fn hex(c: Color32) -> String {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    format!("#{r:02x}{g:02x}{b:02x}/{a:02x}")
}

/// Assert `rows` is the arithmetic progression `start, start+step, …, last` — the measured pitches.
fn assert_pitch(rows: &[usize], start: usize, step: usize, last: usize) {
    let want: Vec<usize> = (start..=last).step_by(step).collect();
    assert_eq!(rows, want, "pitch {step} from {start} to {last}");
}

// ── Hairlines ────────────────────────────────────────────────────────────────

#[test]
fn hairline_is_exactly_one_device_row_at_ppm_1() {
    // img1: the toolbar→content divider, row y=81, x=202..925 (capture width 926).
    let (n, c, recs) = painter_scene(926, 90, 1.0, |p| {
        paint_hairline_h(p, 81.0, 202.0, 925.0, HAIRLINE)
    });
    assert_eq!(n, 1);
    assert_crisp(&recs, 1.0);
    assert_eq!(
        c.rows_of(HAIRLINE),
        vec![81],
        "hairline must occupy one row"
    );
    assert_eq!(c.count(HAIRLINE), 723, "measured span 925 − 202 = 723 px");
    assert_eq!(
        c.painted_total(),
        723,
        "the divider is the only painted thing"
    );
    assert_eq!(c.px(201, 81), Color32::TRANSPARENT, "x0 is exclusive-left");
    assert_eq!(c.px(202, 81), HAIRLINE);
    assert_eq!(c.px(924, 81), HAIRLINE);
    assert_eq!(c.px(925, 81), Color32::TRANSPARENT, "x1 is exclusive-right");
    assert_eq!(
        c.px(202, 80),
        Color32::TRANSPARENT,
        "one row above stays empty"
    );
    assert_eq!(
        c.px(202, 82),
        Color32::TRANSPARENT,
        "one row below stays empty"
    );
}

#[test]
fn hairlines_never_straddle_two_device_pixels_at_ppm_2_and_3() {
    // Fractional logical coordinates: the snapping must land the line inside a single device row
    // (PPM 2) / column (PPM 3) instead of half in each — the PPM-2 leg of spec §6.6.
    let (n, c, recs) = painter_scene(24, 8, 2.0, |p| paint_hairline_h(p, 2.3, 1.4, 9.7, HAIRLINE));
    assert_eq!(n, 1);
    assert_crisp(&recs, 2.0);
    assert_eq!(c.rows_of(HAIRLINE), vec![5], "one device row, no straddle");
    // [1.5, 9.5] pt = device [3, 19): 16 device px wide, in exactly one row.
    assert_pitch(&c.cols_of(HAIRLINE), 3, 1, 18);
    assert_eq!(c.count(HAIRLINE), 16);
    assert_eq!(c.count(Color32::TRANSPARENT), 24 * 8 - 16);

    // Canvas in device px: 30 × 36 = a 10 × 12 pt window at PPM 3, so the line is not clipped.
    let (n, c, recs) = painter_scene(30, 36, 3.0, |p| {
        paint_hairline_v(p, 4.7, 1.0, 9.9, HAIRLINE_ALT)
    });
    assert_eq!(n, 1);
    assert_crisp(&recs, 3.0);
    assert_eq!(
        c.cols_of(HAIRLINE_ALT),
        vec![14],
        "one device column, no straddle"
    );
    assert_pitch(&c.rows_of(HAIRLINE_ALT), 3, 1, 29);
    assert_eq!(c.count(HAIRLINE_ALT), 27);
    assert_eq!(c.count(Color32::TRANSPARENT), 30 * 36 - 27);
}

#[test]
fn dither_pair_alternates_per_device_pixel() {
    // img1 y=81 is a dithered pair (#828790 ×370 ↔ #848494 ×938), so a painted divider alternates
    // the two hexes per pixel; the alternation *period* is still unmeasured (spec §7 item 3).
    let (n, c, recs) = painter_scene(12, 4, 1.0, |p| {
        paint_dither_pair_h(p, 2.0, 1.0, 9.0, HAIRLINE, HAIRLINE_ALT)
    });
    assert_eq!(n, 8, "one shape per device pixel");
    assert_crisp(&recs, 1.0);
    assert_eq!(c.count(Color32::TRANSPARENT), 12 * 4 - 8);
    assert_pitch(&c.cols_of(HAIRLINE), 1, 2, 7);
    assert_pitch(&c.cols_of(HAIRLINE_ALT), 2, 2, 8);
    assert_eq!(c.rows_of(HAIRLINE), vec![2]);
    assert_eq!(c.count(HAIRLINE), 4);
    assert_eq!(c.count(HAIRLINE_ALT), 4);
}

// ── Boxes and frames ─────────────────────────────────────────────────────────

#[test]
fn selection_box_is_81x18_with_the_ring_inside() {
    // Measured tree selection: outer 81 × 18 px = #626262 fill 79 × 16 + 1-px #3C3C3C ring.
    let (n, c, recs) = painter_scene(90, 26, 1.0, |p| {
        paint_selection_box(
            p,
            rect_of_edges(
                4.0,
                4.0,
                4.0 + SELECTION_BOX_W_MEASURED,
                4.0 + SELECTION_BOX_H,
            ),
        )
    });
    assert_eq!(n, 5, "4 ring hairlines + 1 interior fill");
    assert_crisp(&recs, 1.0);
    assert_eq!(c.count(Color32::TRANSPARENT), 90 * 26 - 81 * 18);
    assert_eq!(c.count(BG_SELECTION), 79 * 16, "interior 79 × 16");
    assert_eq!(
        c.count(STROKE_SELECTION),
        194,
        "ring = 2*81 + 2*16 = 194 px"
    );
    assert_eq!(c.px(4, 4), STROKE_SELECTION, "top-left corner is ring");
    assert_eq!(
        c.px(84, 21),
        STROKE_SELECTION,
        "bottom-right corner is ring"
    );
    assert_eq!(c.px(5, 5), BG_SELECTION);
    assert_eq!(c.px(83, 20), BG_SELECTION);
    assert_eq!(
        c.px(85, 21),
        Color32::TRANSPARENT,
        "nothing bleeds outside the box"
    );
    assert_eq!(c.px(4, 22), Color32::TRANSPARENT);
    // The ring crosses *every* row and column, so `rows_of`/`cols_of` are all-rows/all-cols by
    // construction; the runs that prove the 81 × 18 geometry are the exact-count ones.
    assert_eq!(
        c.rows_with_count(STROKE_SELECTION, 81),
        vec![4usize, 21],
        "top and bottom ring rows are 81 px wide"
    );
    assert_eq!(
        c.cols_with_count(STROKE_SELECTION, 18),
        vec![4usize, 84],
        "left and right ring columns are 18 px tall"
    );
}

#[test]
fn dialog_edges_are_the_measured_asymmetric_frame() {
    // img4 frame: top #313131, right #1A1A1A, left+bottom #202020, 1 px each. Corner attribution
    // follows the paint order (top, bottom, left, right); the captures cannot say more (spec §5).
    let (n, c, recs) = painter_scene(100, 60, 1.0, |p| {
        paint_dialog_edges(p, rect_of_edges(0.0, 0.0, 100.0, 60.0))
    });
    assert_eq!(n, 4);
    assert_crisp(&recs, 1.0);
    assert_eq!(c.count(Color32::TRANSPARENT), 98 * 58, "interior untouched");
    assert_eq!(
        c.count(STROKE_FRAME_TOP),
        98,
        "top row minus the two overwritten corners"
    );
    assert_eq!(
        c.count(STROKE_FRAME_RIGHT),
        60,
        "right column wins its corners"
    );
    assert_eq!(c.count(BG_PANEL), 158, "left column 60 + bottom row 98");
    assert_eq!(
        c.px(0, 0),
        BG_PANEL,
        "left hairline overwrites the top-left corner"
    );
    assert_eq!(c.px(99, 0), STROKE_FRAME_RIGHT);
    assert_eq!(c.px(0, 59), BG_PANEL);
    assert_eq!(c.px(99, 59), STROKE_FRAME_RIGHT);
    assert_eq!(c.px(50, 0), STROKE_FRAME_TOP);
    assert_eq!(c.px(50, 59), BG_PANEL);
    assert_eq!(c.px(0, 30), BG_PANEL);
    assert_eq!(c.px(99, 30), STROKE_FRAME_RIGHT);
    assert_eq!(c.px(50, 30), Color32::TRANSPARENT);
}

#[test]
fn search_field_frame_is_white_left_and_gray_other_sides() {
    // img1 search field 149 × 23 px: left border #FFFFFF, top/right/bottom #7A7A7A.
    let (n, c, recs) = painter_scene(149, 23, 1.0, |p| {
        paint_field_frame(p, rect_of_edges(0.0, 0.0, SEARCH_FIELD_W, SEARCH_FIELD_H))
    });
    assert_eq!(n, 4);
    assert_crisp(&recs, 1.0);
    assert_eq!(
        c.count(Color32::TRANSPARENT),
        147 * 21,
        "interior untouched"
    );
    assert_eq!(
        c.count(STROKE_FIELD_LEFT),
        23,
        "whole left column is #FFFFFF"
    );
    assert_eq!(
        c.count(STROKE_FIELD_OTHER),
        340 - 23,
        "ring 2*(149+23)−4 = 340 px minus the 23 px white column"
    );
    assert_eq!(c.px(0, 11), STROKE_FIELD_LEFT);
    assert_eq!(c.px(74, 0), STROKE_FIELD_OTHER);
    assert_eq!(c.px(74, 22), STROKE_FIELD_OTHER);
    assert_eq!(c.px(148, 11), STROKE_FIELD_OTHER);
    assert_eq!(c.px(1, 1), Color32::TRANSPARENT);
}

// ── Graph panels ─────────────────────────────────────────────────────────────

#[test]
fn graph_panel_grid_uses_the_measured_10_by_9_pitch() {
    // img4 panel 1 (150 px tall): black bg, 1-px #008000 grid, column pitch exactly 10.0 px.
    let (n, c, recs) = painter_scene(60, 150, 1.0, |p| {
        paint_graph_panel(p, rect_of_edges(0.0, 0.0, 60.0, 150.0), GRAPH_ROW_PITCH_1)
    });
    assert_eq!(n, 22, "1 fill + 5 interior columns + 16 interior rows");
    assert_crisp(&recs, 1.0);
    assert_eq!(
        c.cols_with_count(GRID_GRAPH, 150),
        vec![10usize, 20, 30, 40, 50],
        "interior vertical lines every 10 px; the panel edge carries no line"
    );
    assert_eq!(
        c.rows_with_count(GRID_GRAPH, 60),
        (9..=144).step_by(9).collect::<Vec<usize>>(),
        "interior horizontal lines at pitch 9 (16 of them)"
    );
    assert_eq!(c.count(BG_VALUE_FIELD), 7_370, "9000 px − the 1630 grid px");
    assert_eq!(c.count(GRID_GRAPH), 1_630);
    assert_eq!(c.px(0, 0), BG_VALUE_FIELD, "no line on the panel edge");
    assert_eq!(c.px(9, 0), BG_VALUE_FIELD, "first line sits one pitch in");
    assert_eq!(c.px(50, 0), GRID_GRAPH);
    assert_eq!(c.px(51, 0), BG_VALUE_FIELD);
    assert_eq!(c.px(0, 149), BG_VALUE_FIELD);
    assert_eq!(
        trace_color(),
        TRACE_GRAPH,
        "trace hex is the measured #FFFF00"
    );
}

#[test]
fn graph_panel_grid_uses_the_measured_10_by_8_pitch_on_the_short_panel() {
    // img4 panel 2 (146 px tall): same column pitch, row pitch 8.
    let (n, c, recs) = painter_scene(60, 146, 1.0, |p| {
        paint_graph_panel(p, rect_of_edges(0.0, 0.0, 60.0, 146.0), GRAPH_ROW_PITCH_2)
    });
    assert_eq!(n, 24, "1 fill + 5 interior columns + 18 interior rows");
    assert_crisp(&recs, 1.0);
    assert_eq!(
        c.cols_with_count(GRID_GRAPH, 146),
        vec![10usize, 20, 30, 40, 50],
        "same column pitch on the 146 px panel"
    );
    assert_eq!(
        c.rows_with_count(GRID_GRAPH, 60),
        (8..=144).step_by(8).collect::<Vec<usize>>(),
        "interior horizontal lines at pitch 8 (18 of them)"
    );
    assert_eq!(c.count(GRID_GRAPH), 1_720);
    assert_eq!(c.count(BG_VALUE_FIELD), 7_040);
    assert_eq!(GRAPH_COL_PITCH, 10.0);
}

// ── Scrollbar ────────────────────────────────────────────────────────────────

#[test]
fn scrollbar_slot_thumb_and_arrow_ink_land_in_measured_colors() {
    // img2: 17-px allocated bar, trough #171717 (outer columns included), thumb #4D4D4D inside the
    // 15-px body, arrow ink #676767 3 px tall centered in a 17-px zone (glyph shape unmeasured).
    let (n, c, recs) = painter_scene(17, 100, 1.0, |p| {
        let mut k = paint_scrollbar_slot(p, rect_of_edges(0.0, 0.0, SCROLLBAR_TOTAL_W, 100.0));
        k += paint_scrollbar_thumb(p, rect_of_edges(1.0, 20.0, 1.0 + SCROLLBAR_BODY_W, 60.0));
        k += paint_scrollbar_arrow_ink(
            p,
            rect_of_edges(0.0, 0.0, SCROLLBAR_ARROW_ZONE, SCROLLBAR_ARROW_ZONE),
        );
        k
    });
    assert_eq!(n, 3);
    assert_crisp(&recs, 1.0);
    assert_eq!(
        c.count(Color32::TRANSPARENT),
        0,
        "the allocated slot is fully painted"
    );
    assert_eq!(c.count(BG_SCROLLBAR_TROUGH), 1_700 - 600 - 45);
    assert_eq!(c.count(BG_SCROLLBAR_THUMB), 15 * 40);
    assert_eq!(
        c.count(GLYPH_SCROLLBAR_ARROW),
        SCROLLBAR_ARROW_GLYPH as usize * (SCROLLBAR_ARROW_ZONE - 2.0) as usize,
        "3 px ink band inset by 1 px on both sides"
    );
    assert_eq!(c.px(0, 50), BG_SCROLLBAR_TROUGH, "outer column is trough");
    assert_eq!(
        c.px(16, 50),
        BG_SCROLLBAR_TROUGH,
        "thumb keeps 1 px of trough on each side"
    );
    assert_eq!(c.px(8, 50), BG_SCROLLBAR_THUMB);
    assert_eq!(c.px(8, 8), GLYPH_SCROLLBAR_ARROW, "ink rows 7,8,9");
    assert_eq!(c.px(8, 6), BG_SCROLLBAR_TROUGH);
    assert_eq!(c.px(0, 8), BG_SCROLLBAR_TROUGH, "ink band starts one px in");
}

// ── Region heights through the real egui layout ──────────────────────────────

#[test]
fn titlebar_toolbar_and_header_strip_rasterize_to_the_measured_row_counts() {
    // 30 px black caption, 48 px toolbar, 24 px two-tone header strip — measured row counts,
    // now proven through egui's own layout (`exact_size` + a flat AIDA64 frame).
    let (_, c, recs) = ui_scene(200, 200, 1.0, |ui| {
        egui::Panel::top("aida64_titlebar")
            .exact_size(TITLEBAR_H)
            .frame(titlebar_frame())
            .show_separator_line(false)
            .show(ui, |_| {});
        egui::Panel::top("aida64_toolbar")
            .exact_size(TOOLBAR_H)
            .frame(toolbar_frame())
            .show_separator_line(false)
            .show(ui, |_| {});
        egui::Panel::top("aida64_header")
            .exact_size(HEADER_ROW_H)
            .frame(header_strip_frame())
            .show_separator_line(false)
            .show(ui, |_| {});
        0
    });
    assert_crisp(&recs, 1.0);
    assert_pitch(&c.rows_spanning_full_width(BG_TITLEBAR), 0, 1, 29);
    assert_pitch(&c.rows_spanning_full_width(BG_PANEL), 30, 1, 77);
    assert_pitch(&c.rows_spanning_full_width(BG_HEADER_STRIP), 78, 1, 101);
    assert_eq!(
        c.count(Color32::TRANSPARENT),
        200 * (200 - 102),
        "rows below stay empty"
    );
    assert_eq!(c.count(BG_TITLEBAR), 200 * 30);
    assert_eq!(c.count(BG_PANEL), 200 * 48);
    assert_eq!(c.count(BG_HEADER_STRIP), 200 * 24);
}

#[test]
fn region_heights_double_with_the_device_scale_at_ppm_2() {
    // At PPM 2 a 30-point titlebar is 60 device rows, aligned to the device grid — the HiDPI leg
    // of spec §6.6 (the reference captures themselves are PPM 1.0).
    let (_, c, recs) = ui_scene(200, 200, 2.0, |ui| {
        egui::Panel::top("aida64_titlebar")
            .exact_size(TITLEBAR_H)
            .frame(titlebar_frame())
            .show_separator_line(false)
            .show(ui, |_| {});
        0
    });
    assert_crisp(&recs, 2.0);
    assert_pitch(&c.rows_spanning_full_width(BG_TITLEBAR), 0, 1, 59);
    assert_eq!(c.count(BG_TITLEBAR), 200 * 60);
    assert_eq!(c.count(Color32::TRANSPARENT), 200 * (200 - 60));
}

#[test]
fn painting_a_mixed_scene_keeps_every_rect_crisp_at_ppm_1_and_2() {
    // Whole paint layer in one pass: the crispness invariants must survive every helper.
    for ppm in [1.0, 2.0] {
        let (n, _canvas, recs) = painter_scene(200, 120, ppm, |p| {
            let mut k = paint_hairline_h(p, 0.0, 0.0, 200.0, HAIRLINE);
            k += paint_dither_pair_h(p, 4.0, 0.0, 20.0, HAIRLINE, HAIRLINE_ALT);
            k += paint_selection_box(p, rect_of_edges(6.0, 8.0, 87.0, 26.0));
            k += paint_dialog_edges(p, rect_of_edges(0.0, 0.0, 200.0, 120.0));
            k += paint_graph_panel(
                p,
                rect_of_edges(100.0, 30.0, 160.0, 120.0),
                GRAPH_ROW_PITCH_1,
            );
            k += paint_scrollbar_slot(p, rect_of_edges(183.0, 8.0, 200.0, 112.0));
            k
        });
        assert_eq!(recs.len(), n);
        assert_crisp(&recs, ppm);
    }
}
