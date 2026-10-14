//! Measured AIDA64-Extreme dark-theme layout metrics, in **points at `pixels_per_point() == 1.0`**.
//!
//! All values come from pixel measurements of the four reference captures
//! (`work/aida64/incoming/ksnip_20261014-003*.png`, 96 DPI `pHYs`, so capture pixel == point at
//! PPM 1.0). The commands that produced each number are recorded in
//! `work/aida64/out/ksnip-20261014-003-ui-spec.md` §1–§5 (tool: `work/aida64/incoming/probe.py`).
//!
//! Conventions used here:
//! * `…_CAPTURE` suffix = the number is an artifact of where the screenshot was cropped, not a
//!   property of the widget (do not treat it as a layout invariant).
//! * `derived:` in a doc-comment = computed from other measured values (documented arithmetic).
//! * `unmeasured:` = no reference pixels existed; the value is a placeholder chosen from
//!   egui defaults and must be re-measured before it is trusted.

/// The captures were taken at 96 DPI with no HiDPI downscale (`pHYs` 96.012 × 96.012), and the
/// two main-window captures matched pixel-for-pixel after a (−2, −1) translation — so every
/// metric below is exact only at this calibration. At PPM 2.0 multiply layout values by PPM and
/// snap to device pixels (`paint::snap`).
pub const CALIBRATION_PPM: f32 = 1.0;

// ── Window chrome ────────────────────────────────────────────────────────────

/// Titlebar height: 30 device px, fill `#000000` (img1 y=3..32; img4 y=1..30 — independent
/// confirmations, `probe.py rband`s + `cmp` gave 0 mismatch for the shared chrome).
pub const TITLEBAR_H: f32 = 30.0;

/// Toolbar height: 48 device px, fill `#202020` (img1 y=33..80; identical in img2).
pub const TOOLBAR_H: f32 = 48.0;

/// Universal stroke width: every measured border/hairline is 1 px thick, in every region of every
/// capture (`probe.py lines`/`runv`).
pub const BORDER_W: f32 = 1.0;

/// Left tree panel width **in the capture** (x=8..201 with the divider at x=202). The real panel is
/// user-resizable, so this is capture context, not an invariant.
pub const TREE_PANEL_W_CAPTURE: f32 = 194.0;

// ── Tree ─────────────────────────────────────────────────────────────────────

/// Row pitch 18.0 px. Independently confirmed against the report by autocorrelating the row
/// profile of the tree band (x=8..201): lag 18 → corr 0.794, lag 17 → 0.730, lag 19 → 0.718.
pub const TREE_ROW_H: f32 = 18.0;

/// Indent per tree level: 19 px (img2: child icon ink starts at x=46, root icon at x=27).
/// egui 0.36 `Style.spacing.indent` is the mechanism that consumes this value.
pub const TREE_INDENT: f32 = 19.0;

/// Tree/category icon box: 16 px (ink measured x=29..44 ⇒ 16 px wide).
pub const TREE_ICON: f32 = 16.0;

/// Chevron (`>` / `v`) glyph slot: 8 px (measured x=12..19 inside the 16 px icon area).
pub const CHEVRON_SLOT: f32 = 8.0;

/// Gap between the icon box and the label: 2 px (`probe.py runh 90`/`runh 107` ink runs).
pub const ICON_LABEL_GAP: f32 = 2.0;

/// Selection box outline: 1 px, hard edges (no feather), drawn *tight* around icon+label.
/// Height equals the row pitch: derived from the measured `#626262` run (interior 79 px × 16 px
/// plus the 1 px outline ⇒ 81 × 18 outer) and the outline pixel count (202 ≈ the 194-px perimeter).
pub const SELECTION_BOX_W_MEASURED: f32 = 81.0;
pub const SELECTION_BOX_H: f32 = 18.0;
pub const SELECTION_STROKE_W: f32 = 1.0;

/// measured (left outset): the selection ring's left edge is x=27 in `ksnip_20261014-003601.png`
/// (spec §1, box 81×18) while the selected row's 16 px glyph crop starts at x=29
/// (`work/aida64/out/atlas-analysis.md` part C: `tree-glyph` x=29..44, y=83..97) ⇒ the box wraps
/// 2 px outside the icon.
/// unmeasured: the *right* inset is assumed symmetric (spec §5 does not itemize the label-ink end).
pub const SELECTION_INSET_X: f32 = 2.0;

/// Tree panel ink starts at x=8 and spans to 201 (spec §1, `ksnip_20261014-003601.png`: panel
/// x=8..201, divider 202). The 8-px window-edge band is the same `#202020` as the window, so it
/// is frame margin geometry, not ink ⇒ encoded as the tree panel's outer left margin.
pub const TREE_PANEL_LEFT_INSET: f32 = 8.0;

/// Row-column inner left pad: the selected row's glyph crop starts at x=29
/// (`work/aida64/out/atlas-analysis.md` part C: tree-glyph x=29..44, y=83..97) while the panel
/// inset is 8 ⇒ 29 − 8 = 21; cross-check with the selection ring left x=27 (spec §1):
/// 8 + 21 − `SELECTION_INSET_X`(2) = 27 ✓. Both input numbers are measurements.
pub const TREE_PANEL_INNER_LEFT: f32 = 21.0;

// ── Scrollbar (img2; classic non-floating) ───────────────────────────────────

/// Total allocated scrollbar width: 17 px = 15 px body + 2 × 1 px `#171717` edge columns.
pub const SCROLLBAR_TOTAL_W: f32 = 17.0;
pub const SCROLLBAR_BODY_W: f32 = 15.0;
pub const SCROLLBAR_EDGE_W: f32 = 1.0;

/// 4 px `#202020` gutter measured between content and the scrollbar/divider region (img2,
/// `runh 300`). Attribution (content→bar vs bar→divider) is not fully resolved, see spec §7.
pub const SCROLLBAR_GUTTER_W: f32 = 4.0;

/// Arrow button zones are ≈17 px tall/wide with a 3 px-tall glyph (`#676767`).
pub const SCROLLBAR_ARROW_ZONE: f32 = 17.0;
pub const SCROLLBAR_ARROW_GLYPH: f32 = 3.0;

// ── Content ribbon (tiles) ───────────────────────────────────────────────────

/// Horizontal pitch between ribbon tiles: 75 px (img1 `runh 110`).
pub const TILE_PITCH_X: f32 = 75.0;

/// Tile icon blob measured 24–26 px tall (y=87..113). An atlas entry is nominal 24 px; the ±2 is
/// AA ramp extent, so keep both bounds.
pub const TILE_ICON_BLOB_MIN: f32 = 24.0;
pub const TILE_ICON_BLOB_MAX: f32 = 26.0;

/// Two-line tile labels: line pitch 15 px, first-line ink top = tile_top + 42 px. Note "ink top"
/// is glyph ink, not the box top — the em box will start a bit higher.
pub const TILE_LABEL_LINE_PITCH: f32 = 15.0;
pub const TILE_LABEL_INK_TOP_OFFSET: f32 = 42.0;

// ── img4 dialog widgets ──────────────────────────────────────────────────────

/// Checkbox square: 13 × 13 px with a 1 px `#D2D2D2` outline; interior = panel background.
pub const CHECKBOX_SIZE: f32 = 13.0;

/// derived: 13 px − 2 × 1 px outline = 11 px of interior; used as the tick glyph box
/// (`Style.spacing.icon_width_inner`). The tick's own ink extent is `unmeasured:`.
pub const CHECKBOX_INTERIOR: f32 = 11.0;

/// Checkbox/list row pitch: 20 px (img4 `runv 40` / `runh 72`) — deliberately different from the
/// tree's 18 px, so row heights must be set per region.
pub const CHECKBOX_ROW_PITCH: f32 = 20.0;

/// Tab strip: row height 19 px. Tab widths for the six img4 tabs (Temperatures … Close-adjacent
/// strip): 83, 81, 56, 48, 50, 58 px. Text-dependent, so treat as the calibration sample set.
pub const TAB_H: f32 = 19.0;
pub const TAB_WIDTHS_MEASURED: [f32; 6] = [83.0, 81.0, 56.0, 48.0, 50.0, 58.0];

/// Log-list header row: 24 px, fill `#191919` while the body stays `#202020`.
pub const HEADER_ROW_H: f32 = 24.0;

/// Button: fill rect 71 × 19 px. derived: outer footprint incl. the 1 px outline = 73 × 21 px.
/// Outline color carries the state (hot `#FFFFFF`, enabled `#9B9B9B`, disabled `#404040`); the
/// fill did *not* change between states, which is what pins `expansion = 0` in the egui style.
pub const BUTTON_W: f32 = 71.0;
pub const BUTTON_H: f32 = 19.0;
pub const BUTTON_FOOTPRINT_W: f32 = BUTTON_W + 2.0 * BORDER_W;
pub const BUTTON_FOOTPRINT_H: f32 = BUTTON_H + 2.0 * BORDER_W;

/// Inter-button gaps measured 9–28 px (grouped buttons; Close sits far right) ⇒ horizontal
/// spacing is set per-button by the layout, not by a global spacing constant.
pub const BUTTON_GAP_MIN: f32 = 9.0;
pub const BUTTON_GAP_MAX: f32 = 28.0;

/// Value fields (img4 right column): height 24 px, fill `#000000`. The x-extents were not
/// measured (`unmeasured:` in spec §7 item 5).
pub const VALUE_FIELD_H: f32 = 24.0;

// ── Graph panels (img4) ──────────────────────────────────────────────────────

/// Vertical grid column pitch: exactly 10.0 px (`probe.py grid` on both panels).
pub const GRAPH_COL_PITCH: f32 = 10.0;

/// Panel 1: height 150 px, horizontal grid pitch 9 px. Panel 2: height 146 px, pitch 8 px.
/// The pitch difference comes from the panel heights, not from a unit change: keep both.
pub const GRAPH_PANEL_H_1: f32 = 150.0;
pub const GRAPH_ROW_PITCH_1: f32 = 9.0;
pub const GRAPH_PANEL_H_2: f32 = 146.0;
pub const GRAPH_ROW_PITCH_2: f32 = 8.0;

/// Gutter between the two stacked graph panels: 4 px.
pub const GRAPH_PANEL_GUTTER: f32 = 4.0;

/// Axis label gutter on the left (x=17..72) and the empty gutter on the right.
pub const GRAPH_AXIS_GUTTER_W: f32 = 56.0;
pub const GRAPH_RIGHT_GUTTER_W: f32 = 89.0;

// ── img3 (Cache & Memory Benchmark) skin ─────────────────────────────────────

/// Numeric table row pitch ≈ 19.6 px: measured separator positions alternate 19/21 px, i.e. the
/// original rasterizes a non-integer pitch exactly like `TableBuilder` row_height 19.6 would.
/// Do **not** round this to 20.0 — that would drift the lines away from the reference.
pub const TABLE_ROW_H: f32 = 19.6;

/// Table outer frame in img3 capture coordinates: x=11..519, y=90..474.
pub const TABLE_FRAME_X0: f32 = 11.0;
pub const TABLE_FRAME_X1: f32 = 519.0;
pub const TABLE_FRAME_Y0: f32 = 90.0;
pub const TABLE_FRAME_Y1: f32 = 474.0;

/// Label/value column divider in img3: x=120.
pub const TABLE_DIVIDER_X: f32 = 120.0;

/// Gradient dither amplitude measured ±4 on the img3 window skin.
pub const GRADIENT_DITHER_AMPLITUDE: f32 = 4.0;

// ── Toolbar / misc widgets ───────────────────────────────────────────────────

/// Search field (img1): 149 × 23 px, left edge `#FFFFFF`, other three edges `#7A7A7A`,
/// interior `#202020`.
pub const SEARCH_FIELD_W: f32 = 149.0;
pub const SEARCH_FIELD_H: f32 = 23.0;

/// Hamburger glyph (img1): 3 bars × 3 px, bar pitch 6.5 px, ≈16 px tall.
pub const HAMBURGER_BAR_W: f32 = 3.0;
pub const HAMBURGER_BAR_PITCH: f32 = 6.5;
pub const HAMBURGER_H: f32 = 16.0;

/// Icon atlas sizes required: 16 px (tree/checkbox/dialog rows) and 24 px (ribbon tiles; the
/// measured blob is 24–26 px where 26 includes the 1 px AA extent, so the atlas cell is the tight
/// 24 px and the AA is painted per sprite). Toolbar icon *ink* measured 21–23 px tall ⇒ atlas must
/// also cover that band. Each size needs a 1× and a 2× variant (PPM 1/2), sRGB, hard edges with
/// their own 1 px AA, no shadow.
pub const ICON_ATLAS_SIZES: [f32; 2] = [16.0, 24.0];
pub const ICON_TOOLBAR_INK_MIN: f32 = 21.0;
pub const ICON_TOOLBAR_INK_MAX: f32 = 23.0;

// ── Typography (metrics-derived, font file unresolved) ───────────────────────

/// Working hypothesis from measured glyph metrics: Segoe UI 9 pt @ 96 DPI ⇒ em = 12 px.
/// measured: cap height 8–9 px for tree/title/tab/label text; 10 px for the big numeric rows;
/// advance per char 4.8–7.6 px depending on the glyph set.
pub const FONT_EM_PTS: f32 = 12.0;
pub const CAP_HEIGHT_MIN: f32 = 8.0;
pub const CAP_HEIGHT_MAX: f32 = 9.0;
pub const CAP_HEIGHT_BIG_NUMERIC: f32 = 10.0;
pub const ADVANCE_PER_CHAR_MIN: f32 = 4.8;
pub const ADVANCE_PER_CHAR_MAX: f32 = 7.6;

/// derived: 18 px tree pitch / 12 px em = 1.5 vertical rhythm. Vertical rhythm is *set per region*
/// (18 tree / 19 tab / 19.6 table / 20 checkbox-list), never derived from `text_styles`.
pub const TREE_RHYTHM_RATIO: f32 = TREE_ROW_H / FONT_EM_PTS;

// Compile-time band guards: if someone edits a constant away from its measured value (or past a
// physically impossible relation), the crate stops building rather than failing a test at runtime.
const _: () = {
    assert!(TABLE_ROW_H > 19.0 && TABLE_ROW_H < 20.0); // 19.6 must stay non-integer
    assert!(FONT_EM_PTS > CAP_HEIGHT_MAX); // em must exceed the measured cap height
    assert!(CAP_HEIGHT_MIN <= CAP_HEIGHT_MAX);
    assert!(ICON_ATLAS_SIZES[0] == TREE_ICON); // 16 px atlas box == measured tree icon box
    assert!(ICON_ATLAS_SIZES[1] == TILE_ICON_BLOB_MIN); // atlas 24 px == tight blob; 26 px is AA extent
    assert!(TILE_ICON_BLOB_MIN <= TILE_ICON_BLOB_MAX);
    assert!(ICON_TOOLBAR_INK_MIN <= ICON_TOOLBAR_INK_MAX);
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derived_values_follow_the_measured_ones() {
        assert_eq!(BUTTON_FOOTPRINT_W, BUTTON_W + 2.0 * BORDER_W);
        assert_eq!(BUTTON_FOOTPRINT_H, BUTTON_H + 2.0 * BORDER_W);
        assert_eq!(SCROLLBAR_TOTAL_W, SCROLLBAR_BODY_W + 2.0 * SCROLLBAR_EDGE_W);
        assert_eq!(CHECKBOX_INTERIOR, CHECKBOX_SIZE - 2.0 * BORDER_W);
        assert_eq!(SELECTION_BOX_H, TREE_ROW_H);
        assert_eq!(SELECTION_STROKE_W, BORDER_W);
        assert_eq!(TREE_RHYTHM_RATIO, 1.5);
    }

    #[test]
    fn measured_values_are_not_silently_rounded() {
        // 19.6 px is a *measured* non-integer pitch (separators rasterize 19/21 in the original);
        // the band that keeps it non-integer is compile-time guarded above.
        assert_eq!(TABLE_ROW_H, 19.6);
        assert_eq!(TABLE_ROW_H.floor(), 19.0); // the value is not 20.0 rounded down at definition
        assert_eq!(GRAPH_COL_PITCH, 10.0);
        assert_eq!(GRAPH_ROW_PITCH_1, 9.0);
        assert_eq!(GRAPH_ROW_PITCH_2, 8.0);
        assert_eq!(HAMBURGER_BAR_PITCH, 6.5);
    }

    #[test]
    fn row_pitches_differ_per_region_as_measured() {
        assert_eq!(TREE_ROW_H, 18.0);
        assert_eq!(TAB_H, 19.0);
        assert_eq!(CHECKBOX_ROW_PITCH, 20.0);
        assert_eq!(HEADER_ROW_H, 24.0);
        assert_eq!(VALUE_FIELD_H, 24.0);
        assert_eq!(GRAPH_PANEL_H_1 - GRAPH_PANEL_H_2, 4.0);
        // The tab width set is the img4 calibration sample (sum is the measured strip width).
        let sum: f32 = TAB_WIDTHS_MEASURED.iter().sum();
        assert_eq!(sum, 376.0);
    }

    #[test]
    fn glyph_metrics_are_inside_the_measured_bands() {
        // The band/ordering relations are compile-time guarded above; here the exact measured
        // endpoints are pinned so an unmeasured edit also fails a test.
        assert_eq!(CAP_HEIGHT_MIN, 8.0);
        assert_eq!(CAP_HEIGHT_MAX, 9.0);
        assert_eq!(CAP_HEIGHT_BIG_NUMERIC, 10.0);
        assert_eq!(FONT_EM_PTS, 12.0);
        assert_eq!(ICON_ATLAS_SIZES[0], TREE_ICON);
        assert_eq!(ICON_ATLAS_SIZES[1], TILE_ICON_BLOB_MIN); // 24 px tile atlas == tight blob (26 = AA extent)
        assert_eq!(ICON_TOOLBAR_INK_MIN, 21.0);
        assert_eq!(ICON_TOOLBAR_INK_MAX, 23.0);
    }
}
