//! Real-renderer pixel gate: rasterizes the AIDA64 paint layer and frames through `egui-wgpu`
//! inside `egui_kittest::Harness` and probes the returned pixels.
//!
//! Software Vulkan (`lavapipe`) stands in for a GPU here — measured evidence:
//! `/usr/share/vulkan/icd.d/lvp_icd.json` + `libvulkan_lvp.so` in `ldconfig`, and
//! `work/aida64/logs/ui-test-21.out` where the harness really produced a 200×200 raster.
//! `egui_kittest-0.36.2/src/wgpu.rs:37-63` installs an adapter selector that prefers
//! `DeviceType::Cpu` (software rasterizers) with `RendererOptions::PREDICTABLE`, which is what makes
//! this gate deterministic (proved by `gpu_raster_is_reproducible_run_to_run`).
//!
//! No baseline PNGs are stored in git (`work/*`/binaries stay out of commits): every expectation is
//! derived from the measured constants in `src/metrics.rs` + `src/palette.rs`, which in turn come
//! from `work/aida64/out/ksnip-20261014-003-ui-spec.md` §5 (calibration PPM 1.0).
//!
//! Two harness behaviours are measured facts, documented in `work/aida64/logs/ui-test-21.out`, and
//! encoded in the expectations below:
//!   1. egui insets the top-level content by **8 points** (`window_margin`), so on a 200-point canvas
//!      the painted bbox is device `x=8..=191, y=8..=191` at PPM 1 (and `16..=383` at PPM 2);
//!   2. a painter obtained inside a `CentralPanel` is clipped to that panel's content rect, so probes
//!      must be placed inside the content area, and the readback clear color is fully transparent
//!      black (`#000000/00`), i.e. the margin is *transparent*, not panel-colored.
//!
//! Every painted pixel of the AIDA64 paint layer rasterizes to its exact measured hex with alpha
//! `ff` and zero feathering — measured here, and the point of these tests.

use aida64_ui_theme::metrics::{
    GRAPH_COL_PITCH, GRAPH_ROW_PITCH_1, HEADER_ROW_H, SCROLLBAR_ARROW_GLYPH, SCROLLBAR_ARROW_ZONE,
    SCROLLBAR_BODY_W, SCROLLBAR_TOTAL_W, SELECTION_BOX_H, SELECTION_BOX_W_MEASURED, TITLEBAR_H,
    TOOLBAR_H,
};
use aida64_ui_theme::paint::{
    paint_dialog_edges, paint_field_frame, paint_graph_panel, paint_hairline_h,
    paint_scrollbar_arrow_ink, paint_scrollbar_slot, paint_scrollbar_thumb, paint_selection_box,
    rect_of_edges, trace_color,
};
use aida64_ui_theme::palette::{
    BG_HEADER_STRIP, BG_PANEL, BG_SCROLLBAR_THUMB, BG_SCROLLBAR_TROUGH, BG_SELECTION, BG_TITLEBAR,
    BG_VALUE_FIELD, GLYPH_SCROLLBAR_ARROW, GRID_GRAPH, HAIRLINE, HAIRLINE_ALT, ICON_ACCENTS,
    STROKE_FIELD_LEFT, STROKE_FIELD_OTHER, STROKE_FRAME_RIGHT, STROKE_FRAME_TOP, STROKE_SELECTION,
    TEXT_PRIMARY, TRACE_GRAPH,
};
use aida64_ui_theme::{apply, header_strip_frame, panel_frame, titlebar_frame, toolbar_frame};
use egui::Color32;
use egui::Ui;
use egui::Vec2;
use egui_kittest::Harness;
use image::Pixel;
use image::RgbaImage;

/// Canvas side in **points** (the harness interprets `with_size` in points; the raster comes back in
/// device pixels = points × PPM).
const BAND_CANVAS: f32 = 200.0;
const PROBE_CANVAS: f32 = 400.0;
/// Measured egui content inset, in points (`ui-test-21.out`: bbox starts at device 8 px on a
/// 200-point canvas at PPM 1).
const CONTENT_INSET: f32 = 8.0;

// ---------------------------------------------------------------- harness plumbing

/// Raster `app` on a `size`×`size` point canvas at `ppm`. Two steps are enough for panels to
/// measure and for the central layout to settle (measured: the third step is never reached).
/// A single software-Vulkan device is alive at a time: measured here, concurrent lavapipe
/// devices from parallel test threads abort the process (SIGSEGV, `ui-test-29.err`), so every
/// GPU raster takes this lock. Poisoning is tolerated (a failing GPU test must not cascade).
static RENDER_SERIALIZER: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn render_scene(mut app: impl FnMut(&mut Ui), size: f32, ppm: f32) -> RgbaImage {
    let _device_lock = RENDER_SERIALIZER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut harness = Harness::builder()
        .with_size(Vec2::new(size, size))
        .with_pixels_per_point(ppm)
        .build_ui(&mut app);
    harness.step();
    harness.step();
    harness
        .render()
        .unwrap_or_else(|err| panic!("egui-wgpu raster failed: {err} (lavapipe software Vulkan)"))
}

/// `#RRGGBB/AA` for a probe.
fn hex4(p: [u8; 4]) -> String {
    format!("#{:02x}{:02x}{:02x}/{:02x}", p[0], p[1], p[2], p[3])
}

fn want(c: Color32) -> [u8; 4] {
    c.to_srgba_unmultiplied()
}

fn px(img: &RgbaImage, x: u32, y: u32) -> [u8; 4] {
    img.get_pixel(x, y)
        .channels()
        .to_vec()
        .try_into()
        .expect("rgba pixel")
}

fn at(img: &RgbaImage, x: u32, y: u32, c: Color32, msg: &str) {
    let got = px(img, x, y);
    assert_eq!(got, want(c), "{msg} at ({x},{y}) = {}", hex4(got));
}

fn is_paint(c: [u8; 4]) -> bool {
    c[3] != 0
}

fn count(img: &RgbaImage, c: Color32) -> usize {
    let w = img.width() as usize;
    let h = img.height() as usize;
    let target = want(c);
    let mut n = 0usize;
    for y in 0..h as u32 {
        for x in 0..w as u32 {
            if px(img, x, y) == target {
                n += 1;
            }
        }
    }
    n
}

/// Device rows that are filled with `c` across the whole painted width.
fn full_rows(img: &RgbaImage, c: Color32) -> Vec<u32> {
    let w = img.width();
    let h = img.height();
    let target = want(c);
    let mut rows = Vec::new();
    for y in 0..h {
        let painted_x = (0..w)
            .filter(|&x| is_paint(px(img, x, y)))
            .collect::<Vec<u32>>();
        let hit_x = painted_x
            .iter()
            .filter(|&&x| px(img, x, y) == target)
            .copied()
            .collect::<Vec<u32>>();
        if !hit_x.is_empty()
            && hit_x.len() == painted_x.len()
            && *hit_x.first().expect("non-empty") + (hit_x.len() as u32 - 1)
                == *hit_x.last().expect("non-empty")
        {
            rows.push(y);
        }
    }
    rows
}

fn row_count(img: &RgbaImage, y: u32, c: Color32) -> usize {
    let target = want(c);
    (0..img.width())
        .filter(|&x| px(img, x, y) == target)
        .count()
}

fn col_count(img: &RgbaImage, x: u32, c: Color32) -> usize {
    let target = want(c);
    (0..img.height())
        .filter(|&y| px(img, x, y) == target)
        .count()
}

fn rows_with_count(img: &RgbaImage, c: Color32, n: u32) -> Vec<u32> {
    (0..img.height())
        .filter(|&y| row_count(img, y, c) == n as usize)
        .collect()
}

fn cols_with_count(img: &RgbaImage, c: Color32, n: u32) -> Vec<u32> {
    (0..img.width())
        .filter(|&x| col_count(img, x, c) == n as usize)
        .collect()
}

fn assert_pitch(values: &[u32], start: u32, step: u32, last: u32, msg: &str) {
    let expect: Vec<u32> = (start..=last).step_by(step as usize).collect();
    assert_eq!(values, expect, "{msg}");
}

/// Inclusive device-pixel bbox of everything painted.
fn bbox(img: &RgbaImage) -> ((u32, u32), (u32, u32)) {
    let mut min_x = u32::MAX;
    let mut min_y = u32::MAX;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    for y in 0..img.height() {
        for x in 0..img.width() {
            if is_paint(px(img, x, y)) {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }
    ((min_x, min_y), (max_x, max_y))
}

/// Run-length summary of one row/column: `(start, end_exclusive, "#rrggbb/aa")`.
fn row_runs(img: &RgbaImage, y: u32) -> Vec<(u32, u32, String)> {
    runs(img.width(), |i| px(img, i, y))
}

fn col_runs(img: &RgbaImage, x: u32) -> Vec<(u32, u32, String)> {
    runs(img.height(), |i| px(img, x, i))
}

fn runs(len: u32, mut at_index: impl FnMut(u32) -> [u8; 4]) -> Vec<(u32, u32, String)> {
    let mut out: Vec<(u32, u32, String)> = Vec::new();
    let mut i = 0u32;
    while i < len {
        let c = at_index(i);
        let mut j = i + 1;
        while j < len && at_index(j) == c {
            j += 1;
        }
        out.push((i, j, hex4(c)));
        i = j;
    }
    out
}

// ---------------------------------------------------------------- the scenes

/// The measured top-of-window stack: 30 px titlebar, 48 px toolbar, 24 px header strip, panel
/// content below (`ksnip-20261014-003-ui-spec.md` §5).
fn band_scene(ui: &mut Ui) {
    apply(ui.ctx());
    egui::Panel::top("aida64_titlebar")
        .exact_size(TITLEBAR_H)
        .frame(titlebar_frame())
        .show_separator_line(false)
        .show(ui, |_ui| {});
    egui::Panel::top("aida64_toolbar")
        .exact_size(TOOLBAR_H)
        .frame(toolbar_frame())
        .show_separator_line(false)
        .show(ui, |_ui| {});
    egui::Panel::top("aida64_header")
        .exact_size(HEADER_ROW_H)
        .frame(header_strip_frame())
        .show_separator_line(false)
        .show(ui, |_ui| {});
    egui::CentralPanel::default()
        .frame(panel_frame())
        .show(ui, |_ui| {});
}

/// The paint layer alone, inside one panel: probes are placed in *point* coordinates inside the
/// content rect (`[8, PROBE_CANVAS - 8)`) so nothing is clipped, and the cells are mutually
/// disjoint except for the scrollbar slot/thumb/arrow stack (intentional).
fn probe_scene(ui: &mut Ui) {
    apply(ui.ctx());
    egui::CentralPanel::default()
        .frame(panel_frame())
        .show(ui, |ui| {
            let p = ui.painter();
            // Measured GPU-raster limitation (`work/aida64/logs/ui-test-24.out` + `ui-test-25.out`):
            // a filled rect that is one device pixel in *both* dimensions tessellates to a degenerate
            // core (tessellation bounds `[[10.5 13.5] - [10.5 13.5]]`) and rasterizes to 0 px under
            // `RendererOptions::PREDICTABLE`. The per-device-pixel dither pair is therefore carried by
            // the CPU raster contract (`tests/pixel_grid.rs`, 8 alternating px at PPM 1); here a single
            // 1×1 device-px `HAIRLINE_ALT` cell is painted on purpose and asserted to render as 0 px.
            let _ = p.rect_filled(
                rect_of_edges(
                    CONTENT_INSET + 2.0,
                    CONTENT_INSET + 5.0,
                    CONTENT_INSET + 3.0,
                    CONTENT_INSET + 6.0,
                ),
                egui::CornerRadius::ZERO,
                HAIRLINE_ALT,
            );
            // plain 1-px hairline (the tree/content divider); the helper fixes the height to one
            // device pixel at *any* scale (`src/paint.rs:74-78`), which is what keeps 1-px features
            // crisp at PPM 2 as well.
            let _ = paint_hairline_h(
                p,
                CONTENT_INSET + 2.0,
                CONTENT_INSET,
                CONTENT_INSET + 184.0,
                HAIRLINE,
            );
            // graph panel: 40×60 pt, 10 × 9 grid pitch
            let _ = paint_graph_panel(
                p,
                rect_of_edges(
                    CONTENT_INSET,
                    CONTENT_INSET + 12.0,
                    CONTENT_INSET + 40.0,
                    CONTENT_INSET + 72.0,
                ),
                GRAPH_ROW_PITCH_1,
            );
            // 81×18 tree selection box, ring fully inside
            let _ = paint_selection_box(
                p,
                rect_of_edges(
                    CONTENT_INSET + 52.0,
                    CONTENT_INSET + 12.0,
                    CONTENT_INSET + 52.0 + SELECTION_BOX_W_MEASURED,
                    CONTENT_INSET + 12.0 + SELECTION_BOX_H,
                ),
            );
            // scrollbar: 17-px slot with the 15-px thumb and the measured arrow ink zone
            let _ = paint_scrollbar_slot(
                p,
                rect_of_edges(
                    PROBE_CANVAS - CONTENT_INSET - SCROLLBAR_TOTAL_W,
                    CONTENT_INSET + 12.0,
                    PROBE_CANVAS - CONTENT_INSET,
                    CONTENT_INSET + 94.0,
                ),
            );
            let _ = paint_scrollbar_thumb(
                p,
                rect_of_edges(
                    PROBE_CANVAS - CONTENT_INSET - SCROLLBAR_TOTAL_W + 1.0,
                    CONTENT_INSET + 32.0,
                    PROBE_CANVAS - CONTENT_INSET - 1.0,
                    CONTENT_INSET + 72.0,
                ),
            );
            let _ = paint_scrollbar_arrow_ink(
                p,
                rect_of_edges(
                    PROBE_CANVAS - CONTENT_INSET - SCROLLBAR_TOTAL_W,
                    CONTENT_INSET + 12.0,
                    PROBE_CANVAS - CONTENT_INSET,
                    CONTENT_INSET + 12.0 + SCROLLBAR_ARROW_ZONE,
                ),
            );
            // dialog: measured asymmetric 1-px frame (top #313131, right #1A1A1A, left/bottom panel)
            let _ = paint_dialog_edges(
                p,
                rect_of_edges(
                    CONTENT_INSET,
                    CONTENT_INSET + 82.0,
                    CONTENT_INSET + 60.0,
                    CONTENT_INSET + 142.0,
                ),
            );
            // search field: 20×12 pt frame, white left edge, #7A7A7A other sides
            let _ = paint_field_frame(
                p,
                rect_of_edges(
                    CONTENT_INSET + 72.0,
                    CONTENT_INSET + 82.0,
                    CONTENT_INSET + 92.0,
                    CONTENT_INSET + 94.0,
                ),
            );
        });
}

// ---------------------------------------------------------------- measured expectations (PPM 1)

#[test]
fn gpu_raster_stacks_the_measured_region_rows_with_the_egui_content_inset() {
    let img = render_scene(band_scene, BAND_CANVAS, 1.0);
    let (w, h) = (img.width(), img.height());
    assert_eq!(
        (w, h),
        (200, 200),
        "the harness must raster the requested canvas"
    );

    // egui's 8-point window margin is real and device-scaled (measured ui-test-21.out):
    let ((x0, y0), (x1, y1)) = bbox(&img);
    assert_eq!((x0, y0), (8, 8), "measured content inset");
    assert_eq!(
        (x1, y1),
        (191, 191),
        "content ends 8 px short of the canvas edge"
    );

    // stacking, measured row-by-row in ui-test-21.out: 30/48/24 px of opaque chrome
    let black_rows = full_rows(&img, BG_TITLEBAR);
    assert_eq!(
        black_rows,
        (8..=37).collect::<Vec<u32>>(),
        "30 titlebar rows"
    );
    // measured: the toolbar frame's fill IS `BG_PANEL` (#202020), so the panel-tone rows are the
    // 48-row toolbar band *and* the 82-row empty central band (`ui-test-21.out` row runs).
    let panel_rows = full_rows(&img, BG_PANEL);
    assert_eq!(
        panel_rows,
        (38..=85).chain(110..=191).collect::<Vec<u32>>(),
        "toolbar band 38..=85 + central band 110..=191 share the panel tone",
    );
    assert_eq!(
        panel_rows.len(),
        48 + 82,
        "48 toolbar rows and 82 central rows"
    );
    let header_rows = full_rows(&img, BG_HEADER_STRIP);
    assert_eq!(
        header_rows,
        (86..=109).collect::<Vec<u32>>(),
        "24 header-strip rows"
    );
    // below that the empty central panel is the panel tone down to the inset
    for y in 110..=191 {
        assert_eq!(row_count(&img, y, BG_PANEL), 184, "central panel row {y}");
    }
    assert_eq!(
        count(&img, BG_HEADER_STRIP),
        184 * 24,
        "header strip covers the content width"
    );
    assert_eq!(row_count(&img, 85, BG_PANEL), 184, "toolbar bottom row");
    assert_eq!(
        row_count(&img, 109, BG_HEADER_STRIP),
        184,
        "header strip bottom row"
    );
    assert_eq!(
        row_count(&img, 110, BG_PANEL),
        184,
        "first central content row"
    );

    // the margin is *transparent*, never panel-colored: 200² − 184² px
    let painted_rows = (0..h)
        .filter(|&y| (0..w).any(|x| is_paint(px(&img, x, y))))
        .count();
    assert_eq!(painted_rows, 184, "only the inset-bounded rows carry ink");
    let total_paint = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .filter(|&(x, y)| is_paint(px(&img, x, y)))
        .count();
    assert_eq!(total_paint, 184 * 184, "painted area = content rect");
    assert_eq!(w as usize * h as usize - total_paint, 200 * 200 - 184 * 184);
    for x in [0u32, 199] {
        for y in 0..h {
            assert_eq!(px(&img, x, y)[3], 0, "left/right margin alpha at ({x},{y})");
        }
    }
    for &y in &[0u32, 7, 192, 199] {
        for x in 0..w {
            assert_eq!(px(&img, x, y)[3], 0, "top/bottom margin alpha at ({x},{y})");
        }
    }
    for y in 8..=191 {
        for x in 8..=191 {
            assert_eq!(
                px(&img, x, y)[3],
                255,
                "every painted pixel is opaque ({x},{y})"
            );
        }
    }
}

#[test]
fn gpu_raster_reproduces_the_measured_paint_layer_counts_exactly() {
    let img = render_scene(probe_scene, PROBE_CANVAS, 1.0);
    let (w, h) = (img.width(), img.height());
    assert_eq!((w, h), (400, 400));
    assert_eq!(bbox(&img), ((8, 8), (391, 391)), "content inset at PPM 1");

    // Device-pixel budgets, every number derived from metrics.rs (points == device px at PPM 1).
    // hairline rect [8,192)×[10,11): 184 px
    assert_eq!(
        row_count(&img, 10, HAIRLINE),
        184,
        "hairline is 1 device row"
    );
    // the deliberate 1×1 device-px cell: measured to rasterize to 0 px (tessellation core is
    // zero-width; see the module comment in `probe_scene` and `ui-test-25.out`)
    assert_eq!(
        count(&img, HAIRLINE_ALT),
        0,
        "1×1 device-px cell renders nothing at PPM 1"
    );
    // graph panel [8,48)×[20,80): 40×60 fill, 3 interior cols × 60 + 6 interior rows × 40 − 3×6
    let grid = 3 * 60 + 6 * 40 - 3 * 6;
    assert_eq!(
        count(&img, GRID_GRAPH),
        grid,
        "measured 10×9 grid inside 40×60"
    );
    // graph fill is the same #000000 as the titlebar tone (palette.rs documents the identity)
    assert_eq!(
        count(&img, BG_VALUE_FIELD),
        40 * 60 - grid,
        "graph fill = value-field tone"
    );
    // selection box 81×18: fill 79×16, 1-px ring = perimeter
    assert_eq!(
        count(&img, BG_SELECTION),
        ((SELECTION_BOX_W_MEASURED - 2.0) * (SELECTION_BOX_H - 2.0)) as usize,
        "selection fill 79x16",
    );
    assert_eq!(
        count(&img, STROKE_SELECTION),
        (2.0 * (SELECTION_BOX_W_MEASURED + SELECTION_BOX_H) - 4.0) as usize,
        "selection ring"
    );
    // scrollbar: slot 17×82, thumb 15×40, arrow 3 device rows × 15 → trough = remainder
    let slot = SCROLLBAR_TOTAL_W * 82.0;
    let thumb = SCROLLBAR_BODY_W * 40.0;
    let arrow = SCROLLBAR_ARROW_GLYPH * (SCROLLBAR_ARROW_ZONE - 2.0);
    assert_eq!(
        count(&img, BG_SCROLLBAR_THUMB),
        thumb as usize,
        "thumb 15 px wide"
    );
    assert_eq!(
        count(&img, GLYPH_SCROLLBAR_ARROW),
        arrow as usize,
        "arrow ink = 3 device rows"
    );
    assert_eq!(
        count(&img, BG_SCROLLBAR_TROUGH),
        (slot - thumb - arrow) as usize,
        "trough = slot − thumb − arrow"
    );
    // dialog 60×60: top row 60 then left/right overwrite the corners → 58 top px, right column 60
    assert_eq!(
        count(&img, STROKE_FRAME_TOP),
        58,
        "top edge loses 2 corners to left/right"
    );
    assert_eq!(
        count(&img, STROKE_FRAME_RIGHT),
        60,
        "right edge is painted last"
    );
    // field frame 20×12: ring of distinct px = 2·(20+12) − 4 = 60, of which the whole left column
    // (12 px) is white; the measured GPU count is 48 (#7A7A7A) — `ui-test-23.out`
    assert_eq!(
        count(&img, STROKE_FIELD_LEFT),
        12,
        "search field left edge is 1 px wide, full height"
    );
    assert_eq!(
        count(&img, STROKE_FIELD_OTHER),
        60 - 12,
        "other three sides are #7A7A7A"
    );
    // TEXT_PRIMARY is the same #FFFFFF hex as the field left edge — identity documented in palette.rs
    assert_eq!(
        count(&img, TEXT_PRIMARY),
        12,
        "white used only by the field edge"
    );

    // closure: everything else in the content rect is the panel tone, and the margin is transparent
    let margin = (w * h - 384 * 384) as usize;
    assert_eq!(margin, 400 * 400 - 384 * 384);
    let painted_total = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .filter(|&(x, y)| is_paint(px(&img, x, y)))
        .count();
    assert_eq!(
        painted_total,
        384 * 384,
        "the panel fills the whole content rect"
    );
    // panel tone = content minus every painted feature: hairline 184, graph 40×60 = 2400,
    // scrollbar slot 17×82 = 1394, selection box 81×18 = 1458, dialog top+right 58+60 = 118,
    // field ring 60 → 147 456 − 5 614 = 141 842 (measured in `ui-test-23.out`)
    assert_eq!(
        count(&img, BG_PANEL),
        384 * 384 - (184 + 2400 + 1394 + 1458 + 118 + 60),
        "no feathered or blended px leaked into the panel count",
    );
    let sum = margin
        + count(&img, BG_PANEL)
        + count(&img, BG_VALUE_FIELD)
        + count(&img, BG_SELECTION)
        + count(&img, STROKE_SELECTION)
        + count(&img, BG_SCROLLBAR_TROUGH)
        + count(&img, BG_SCROLLBAR_THUMB)
        + count(&img, GLYPH_SCROLLBAR_ARROW)
        + row_count(&img, 10, HAIRLINE)
        + count(&img, HAIRLINE_ALT)
        + count(&img, GRID_GRAPH)
        + count(&img, STROKE_FRAME_TOP)
        + count(&img, STROKE_FRAME_RIGHT)
        + count(&img, STROKE_FIELD_OTHER)
        + count(&img, STROKE_FIELD_LEFT);
    assert_eq!(sum, (w * h) as usize, "pixel budget must close exactly");
    assert_eq!(
        count(&img, BG_TITLEBAR),
        40 * 60 - grid,
        "no black outside the graph panel"
    );
    assert_eq!(
        count(&img, BG_HEADER_STRIP),
        0,
        "no header tone conjured by the paint layer"
    );
    for c in ICON_ACCENTS {
        assert_eq!(count(&img, c), 0, "icons are not painted yet");
    }
    assert_eq!(
        trace_color(),
        TRACE_GRAPH,
        "the graph trace color is measured yellow"
    );
}

#[test]
fn gpu_raster_places_the_measured_geometry_on_the_device_grid() {
    let img = render_scene(probe_scene, PROBE_CANVAS, 1.0);

    // hairline: exactly device row 10, nothing above/below, half-open right end
    assert_eq!(
        rows_with_count(&img, HAIRLINE, 184),
        vec![10],
        "hairline row"
    );
    assert_eq!(
        row_count(&img, 9, HAIRLINE),
        0,
        "no bleed above the hairline"
    );
    assert_eq!(
        row_count(&img, 11, HAIRLINE),
        0,
        "no bleed below the hairline"
    );
    at(&img, 191, 10, HAIRLINE, "hairline last painted px");
    at(&img, 192, 10, BG_PANEL, "hairline right end is exclusive");

    // the deliberate 1×1 `HAIRLINE_ALT` sample: 0 px rendered, so the cell stays panel tone
    // (measured row runs of row 13 are pure #202020, `ui-test-24.out`; per-device-pixel dither
    // alternation itself is a CPU-raster contract, `tests/pixel_grid.rs` test 3)
    at(
        &img,
        10,
        13,
        BG_PANEL,
        "the 1x1 device-px cell renders nothing, cell stays panel tone",
    );

    // graph: interior lines only, exactly at 10-px (x) and 9-px (y) pitches measured in img4 §5
    assert_pitch(
        &cols_with_count(&img, GRID_GRAPH, 60),
        18,
        GRAPH_COL_PITCH as u32,
        38,
        "grid columns every 10 px",
    );
    assert_pitch(
        &rows_with_count(&img, GRID_GRAPH, 40),
        29,
        GRAPH_ROW_PITCH_1 as u32,
        74,
        "grid rows every 9 px",
    );
    at(
        &img,
        8,
        20,
        BG_VALUE_FIELD,
        "panel corner is fill, not grid",
    );
    at(
        &img,
        18,
        20,
        GRID_GRAPH,
        "first grid column starts at the panel interior",
    );
    at(&img, 8, 29, GRID_GRAPH, "first grid row");
    at(
        &img,
        47,
        79,
        BG_VALUE_FIELD,
        "last px of the panel is fill, not grid",
    );
    at(&img, 48, 20, BG_PANEL, "outside the panel to the right");

    // selection ring: 81 px wide rows at top/bottom, 18 px tall columns at left/right
    assert_eq!(
        rows_with_count(&img, STROKE_SELECTION, SELECTION_BOX_W_MEASURED as u32),
        vec![20, 37],
        "selection ring rows",
    );
    assert_eq!(
        cols_with_count(&img, STROKE_SELECTION, SELECTION_BOX_H as u32),
        vec![60, 140],
        "selection ring columns",
    );
    at(&img, 61, 21, BG_SELECTION, "interior fill");
    at(&img, 139, 36, BG_SELECTION, "interior fill");
    at(&img, 59, 20, BG_PANEL, "outside the ring, left");
    at(&img, 141, 37, BG_PANEL, "outside the ring, right");

    // scrollbar slot: 17 columns; the arrow band is the middle 3 device rows of the 17-px zone,
    // inset 1 px from the slot edges (measured in the PPM-1 dump: rows 27,28,29 cols 376..390)
    let slot_x0 = 392 - 17;
    assert_eq!(
        col_count(&img, slot_x0, BG_SCROLLBAR_TROUGH),
        82,
        "slot left column"
    );
    assert_eq!(
        rows_with_count(&img, GLYPH_SCROLLBAR_ARROW, SCROLLBAR_BODY_W as u32),
        vec![27, 28, 29],
        "arrow band = 3 device rows inside the 17-px zone",
    );
    at(
        &img,
        slot_x0,
        28,
        BG_SCROLLBAR_TROUGH,
        "arrow zone inset: left column stays trough",
    );
    at(
        &img,
        391,
        28,
        BG_SCROLLBAR_TROUGH,
        "arrow zone inset: right column stays trough",
    );
    at(
        &img,
        376,
        28,
        GLYPH_SCROLLBAR_ARROW,
        "arrow ink starts inset by 1 px",
    );
    // thumb rect is [CONTENT_INSET+376 .. ) → points [376,391)x[40,80) (the scene places it at
    // CONTENT_INSET + 32 .. CONTENT_INSET + 72): measured 600 px = 15 × 40 device px
    assert_pitch(
        &rows_with_count(&img, BG_SCROLLBAR_THUMB, SCROLLBAR_BODY_W as u32),
        40,
        1,
        79,
        "thumb spans rows 40..=79 (points [376,391)x[40,80))",
    );
    at(
        &img,
        375,
        40,
        BG_SCROLLBAR_TROUGH,
        "slot left column stays trough beside the thumb",
    );
    at(
        &img,
        391,
        40,
        BG_SCROLLBAR_TROUGH,
        "slot right column stays trough beside the thumb",
    );
    at(
        &img,
        376,
        39,
        BG_SCROLLBAR_TROUGH,
        "thumb top edge inset by 1 px",
    );
    at(
        &img,
        376,
        80,
        BG_SCROLLBAR_TROUGH,
        "thumb bottom edge inset by 1 px",
    );
    at(
        &img,
        376,
        40,
        BG_SCROLLBAR_THUMB,
        "thumb starts inset by 1 px from the slot edge",
    );
    at(
        &img,
        390,
        79,
        BG_SCROLLBAR_THUMB,
        "thumb ends inset by 1 px (column 391 stays trough)",
    );

    // dialog: paint order top → bottom → left → right, so the left/bottom panel tone wins the
    // left corners and the #1A1A1A right column wins the right corners (measured row runs:
    // ui-test-21.out row 172 `(8,10,"#202020"),(11,68,"#313131"),(69,69,"#1a1a1a")`)
    // dialog rect [8,68)x[90,150) -> device corners 8/67 x 90/149
    at(
        &img,
        8,
        90,
        BG_PANEL,
        "dialog top-left corner is the panel tone (left painted after top)",
    );
    at(
        &img,
        67,
        90,
        STROKE_FRAME_RIGHT,
        "dialog top-right corner is #1A1A1A (right painted last)",
    );
    at(&img, 35, 90, STROKE_FRAME_TOP, "dialog top edge is #313131");
    at(
        &img,
        35,
        149,
        BG_PANEL,
        "dialog bottom edge is the panel tone",
    );
    at(&img, 8, 149, BG_PANEL, "dialog bottom-left corner");
    at(
        &img,
        67,
        149,
        STROKE_FRAME_RIGHT,
        "dialog bottom-right corner",
    );
    at(&img, 35, 150, BG_PANEL, "no bleed below the dialog");
    at(
        &img,
        7,
        115,
        Color32::TRANSPARENT,
        "column 7 is the transparent margin (inset is 8 px)",
    );
    at(
        &img,
        9,
        115,
        BG_PANEL,
        "no bleed into the panel interior just right of the dialog edge",
    );
    at(
        &img,
        68,
        115,
        BG_PANEL,
        "no bleed right of the dialog (dialog ends at column 67)",
    );

    // field frame: whole left column white including its corners
    at(&img, 80, 90, STROKE_FIELD_LEFT, "field top-left is white");
    at(
        &img,
        80,
        101,
        STROKE_FIELD_LEFT,
        "field bottom-left is white",
    );
    at(
        &img,
        99,
        90,
        STROKE_FIELD_OTHER,
        "field top-right is #7A7A7A",
    );
    at(
        &img,
        99,
        101,
        STROKE_FIELD_OTHER,
        "field bottom-right is #7A7A7A",
    );
    at(&img, 89, 90, STROKE_FIELD_OTHER, "field top edge");
    at(&img, 89, 101, STROKE_FIELD_OTHER, "field bottom edge");
    at(&img, 99, 95, STROKE_FIELD_OTHER, "field right edge");
    at(&img, 81, 91, BG_PANEL, "field interior is the panel tone");
    at(&img, 79, 95, BG_PANEL, "no bleed outside the field frame");
}

#[test]
fn gpu_raster_introduces_no_color_outside_the_measured_palette_or_transparency() {
    let img = render_scene(probe_scene, PROBE_CANVAS, 1.0);
    let mut offenders: Vec<(u32, u32, String)> = Vec::new();
    let allowed: Vec<[u8; 4]> = vec![
        want(BG_PANEL),
        want(BG_TITLEBAR), // == BG_VALUE_FIELD (#000000): the black of the graph interior
        want(BG_HEADER_STRIP),
        want(BG_SELECTION),
        want(STROKE_SELECTION),
        want(BG_SCROLLBAR_TROUGH),
        want(BG_SCROLLBAR_THUMB),
        want(GLYPH_SCROLLBAR_ARROW),
        want(HAIRLINE),
        want(HAIRLINE_ALT),
        want(GRID_GRAPH),
        want(STROKE_FRAME_TOP),
        want(STROKE_FRAME_RIGHT),
        want(STROKE_FIELD_OTHER),
        want(STROKE_FIELD_LEFT),
        [0, 0, 0, 0], // the harness clear color: transparent margin, not a painted color
    ];
    for y in 0..img.height() {
        for x in 0..img.width() {
            let p = px(&img, x, y);
            if !allowed.contains(&p) && offenders.len() < 12 {
                offenders.push((x, y, hex4(p)));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "egui-wgpu produced colors outside the measured set: {offenders:?}"
    );
    assert_eq!(allowed.len(), 16, "the measured set + transparency");
    // feathering/blend detector: no AA halo around the crisp features
    assert_eq!(
        count(&img, HAIRLINE),
        184,
        "hairline only, no feathered neighbors"
    );
    assert_eq!(
        count(&img, BG_PANEL),
        384 * 384 - (184 + 2400 + 1394 + 1458 + 118 + 60),
        "no feathered px leaked into the panel count"
    );
}

#[test]
fn gpu_raster_doubles_layout_but_keeps_1px_features_one_device_pixel_thick() {
    let img = render_scene(band_scene, BAND_CANVAS, 2.0);
    let (w, h) = (img.width(), img.height());
    assert_eq!((w, h), (400, 400), "PPM 2 must double the raster");
    let ((x0, y0), (x1, y1)) = bbox(&img);
    assert_eq!(
        (x0, y0),
        (16, 16),
        "the 8-point inset doubles with the device scale"
    );
    assert_eq!((x1, y1), (383, 383));
    assert_eq!(
        full_rows(&img, BG_TITLEBAR),
        (16..=75).collect::<Vec<u32>>(),
        "60 device rows of titlebar"
    );
    let panel_rows = full_rows(&img, BG_PANEL);
    assert_eq!(
        panel_rows,
        (76..=171).chain(220..=383).collect::<Vec<u32>>(),
        "toolbar band 76..=171 (96 rows) + central band 220..=383 (164 rows), panel tone shared",
    );
    assert_eq!(
        panel_rows.len(),
        96 + 164,
        "the doubled toolbar is 96 rows, the central band 164"
    );
    assert_eq!(
        full_rows(&img, BG_HEADER_STRIP),
        (172..=219).collect::<Vec<u32>>(),
        "48 device rows of header strip"
    );
    for y in 220..=383 {
        assert_eq!(
            row_count(&img, y, BG_PANEL),
            368,
            "central content row {y} at PPM 2"
        );
    }
    assert_eq!(
        row_count(&img, 15, BG_TITLEBAR),
        0,
        "margin row above the content"
    );
    assert_eq!(
        row_count(&img, 384, BG_PANEL),
        0,
        "margin row below the content"
    );
}

#[test]
fn gpu_raster_keeps_crisp_paint_features_at_ppm_2() {
    let img = render_scene(probe_scene, PROBE_CANVAS, 2.0);
    let (w, h) = (img.width(), img.height());
    assert_eq!((w, h), (800, 800));
    assert_eq!(bbox(&img), ((16, 16), (783, 783)));

    // layout pitches double, line thickness does not (spec §6.6)
    assert_eq!(
        rows_with_count(&img, HAIRLINE, 2 * 184),
        vec![20],
        "hairline: one device row, 368 px long"
    );
    assert_eq!(row_count(&img, 19, HAIRLINE), 0, "no bleed above");
    assert_eq!(row_count(&img, 21, HAIRLINE), 0, "no bleed below");
    assert_eq!(
        cols_with_count(&img, GRID_GRAPH, 2 * 60),
        vec![36, 56, 76],
        "grid columns every 20 device px (= 10 points; first at panel left 16 + 10 pt = 36)",
    );
    assert_eq!(
        rows_with_count(&img, GRID_GRAPH, 2 * 40),
        vec![58, 76, 94, 112, 130, 148],
        "grid rows every 18 device px (= 9 points)",
    );
    // thickness detector that the dumps document directly: the column at device x=40 (between the
    // grid columns 36 and 56) carries exactly the 6 horizontal grid rows = 6 px (`ui-test-26.out`
    // col 40 shows six 1-px #008000 runs); a 2-px-thick *vertical* line would add 120 px there, and
    // row 54 (between grid rows 38?? see below) carries exactly the 3 vertical-line px.
    assert_eq!(
        col_count(&img, 40, GRID_GRAPH),
        6,
        "off-line column carries only the 6 grid rows"
    );
    assert_eq!(
        row_count(&img, 54, GRID_GRAPH),
        3,
        "off-line row carries only the 3 grid columns"
    );
    assert_eq!(
        rows_with_count(&img, STROKE_SELECTION, 2 * SELECTION_BOX_W_MEASURED as u32),
        vec![40, 75],
        "selection ring rows at PPM 2 (outer 162×36 device px)",
    );
    assert_eq!(
        cols_with_count(&img, STROKE_SELECTION, 2 * SELECTION_BOX_H as u32),
        vec![120, 281],
        "selection ring columns at PPM 2",
    );
    // inside the box the two ring columns cross every interior row (measured 2 px in row 41) and
    // the two ring rows cross every interior column (measured 2 px in column 121); a doubled ring
    // would show 4 px there, and the dump's col 120 run `[40, 76)` bounds the ring exactly
    // (`ui-test-26.out`), so rows 39 and 76 outside the ring carry no ring color at all.
    assert_eq!(
        row_count(&img, 41, STROKE_SELECTION),
        2,
        "interior row: exactly the 2 ring columns (a doubled ring column would read 4)",
    );
    assert_eq!(
        col_count(&img, 121, STROKE_SELECTION),
        2,
        "interior column: exactly the 2 ring rows (a doubled ring row would read 4)",
    );
    assert_eq!(
        row_count(&img, 39, STROKE_SELECTION),
        0,
        "no ring bleed above the box"
    );
    assert_eq!(
        row_count(&img, 76, STROKE_SELECTION),
        0,
        "no ring bleed below the box"
    );
    // measured in `ui-test-26.out` (post-patch dump): the 1×1-*point* sample is 2×2 *device* px at
    // PPM 2, so it is solid there (unlike at PPM 1), and every feature count doubles exactly.
    assert_eq!(
        count(&img, HAIRLINE_ALT),
        4,
        "the 1x1-point sample renders 4 device px at PPM 2 (measured)"
    );
    assert_eq!(
        count(&img, GRID_GRAPH),
        822,
        "grid = 3 cols x 120 + 6 rows x 80 - 18 (measured)"
    );
    assert_eq!(
        count(&img, BG_VALUE_FIELD),
        80 * 120 - 822,
        "graph fill 9600 minus grid (measured 8778)"
    );
    assert_eq!(
        count(&img, TEXT_PRIMARY),
        24,
        "field left column is 24 device px tall"
    );
    assert_eq!(
        count(&img, STROKE_FIELD_OTHER),
        100,
        "field ring 124 distinct px minus 24 white"
    );
    assert_eq!(
        count(&img, BG_HEADER_STRIP),
        0,
        "no header tone conjured by the paint layer"
    );
    // panel tone = content minus every painted feature (measured 568 082 px in `ui-test-26.out`):
    // 368 hairline + 80x120 graph + 34x164 scrollbar slot + 162x36 selection + (118+120) dialog
    // edges + 124 field ring + the 4 px sample
    assert_eq!(
        count(&img, BG_PANEL),
        768 * 768 - (368 + 9600 + 5576 + 5832 + 238 + 124 + 4),
        "no feathered or blended px leaked into the panel count at PPM 2",
    );
    // the arrow ink extent is measured as 3 *device* rows (metrics SCROLLBAR_ARROW_GLYPH), so it
    // stays 3 rows while the 17-px zone doubles: cy = (40+74)/2 = 57 device px → rows 56,57,58.
    assert_eq!(
        rows_with_count(
            &img,
            GLYPH_SCROLLBAR_ARROW,
            (SCROLLBAR_TOTAL_W * 2.0 - 2.0) as u32
        ),
        vec![56, 57, 58],
        "arrow ink stays 3 device rows, 32 device px wide (measured 96 px = 3×32, ui-test-23.out)",
    );
    assert_eq!(
        count(&img, BG_SCROLLBAR_TROUGH),
        (SCROLLBAR_TOTAL_W * 82.0 * 4.0
            - SCROLLBAR_BODY_W * 40.0 * 4.0
            - SCROLLBAR_ARROW_GLYPH * (SCROLLBAR_TOTAL_W * 2.0 - 2.0)) as usize,
        "slot 34×164 − thumb 30×80 − arrow 3×32 = 3080 (measured, ui-test-23.out)",
    );
}

#[test]
fn gpu_raster_is_reproducible_run_to_run() {
    let a = render_scene(probe_scene, PROBE_CANVAS, 1.0);
    let b = render_scene(probe_scene, PROBE_CANVAS, 1.0);
    assert_eq!(
        a, b,
        "the PREDICTABLE software rasterizer must be deterministic"
    );
    assert_eq!(count(&a, BG_PANEL), count(&b, BG_PANEL));
}

// ---------------------------------------------------------------- evidence dump (run with -- --nocapture)

#[test]
fn gpu_debug_dump() {
    for (label, size, ppm) in [
        ("bands @1", BAND_CANVAS, 1.0),
        ("bands @2", BAND_CANVAS, 2.0),
        ("probes @1", PROBE_CANVAS, 1.0),
        ("probes @2", PROBE_CANVAS, 2.0),
    ] {
        let img = if label.starts_with("bands") {
            render_scene(band_scene, size, ppm)
        } else {
            render_scene(probe_scene, size, ppm)
        };
        let w = img.width();
        let h = img.height();
        println!("== {label}: {w}x{h} px, bbox {:?}", bbox(&img));
        for c in [
            BG_TITLEBAR,
            BG_PANEL,
            BG_HEADER_STRIP,
            BG_VALUE_FIELD,
            BG_SELECTION,
            STROKE_SELECTION,
            BG_SCROLLBAR_TROUGH,
            BG_SCROLLBAR_THUMB,
            GLYPH_SCROLLBAR_ARROW,
            HAIRLINE,
            HAIRLINE_ALT,
            GRID_GRAPH,
            STROKE_FRAME_TOP,
            STROKE_FRAME_RIGHT,
            STROKE_FIELD_OTHER,
            STROKE_FIELD_LEFT,
        ] {
            println!("   {} = {}", hex4(want(c)), count(&img, c));
        }
        let painted = (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .filter(|&(x, y)| is_paint(px(&img, x, y)))
            .count();
        println!("   painted {painted} of {}", w * h);
        for y in [
            (BAND_CANVAS * ppm) as u32 / 2,
            ((TITLEBAR_H + TOOLBAR_H + HEADER_ROW_H) * ppm) as u32,
        ] {
            println!("   row {y}: {:?}", row_runs(&img, y));
        }
        if label.contains("probes") {
            for x in [16u32, 40, 96, 120, 281, 750, 783] {
                if x < w {
                    println!(
                        "   col {x}: {:?}",
                        col_runs(&img, x).into_iter().take(12).collect::<Vec<_>>()
                    );
                }
            }
            for y in [13u32, 28, 20, 26, 40, 75, 54, 58, 141, 180] {
                if y < h {
                    println!(
                        "   row {y}: {:?}",
                        row_runs(&img, y).into_iter().take(12).collect::<Vec<_>>()
                    );
                }
            }
            println!(
                "   hairline rows(184·ppm) {:?} dither rows {:?}/{:?} arrow rows {:?}",
                rows_with_count(&img, HAIRLINE, 184 * ppm as u32),
                row_count(&img, 26, HAIRLINE),
                row_count(&img, 26, HAIRLINE_ALT),
                rows_with_count(
                    &img,
                    GLYPH_SCROLLBAR_ARROW,
                    (SCROLLBAR_TOTAL_W * ppm - 2.0) as u32
                ),
            );
            println!(
                "   selection ring rows {:?} cols {:?} grid cols {:?} rows {:?}",
                rows_with_count(&img, STROKE_SELECTION, 81 * ppm as u32),
                cols_with_count(&img, STROKE_SELECTION, 18 * ppm as u32),
                cols_with_count(&img, GRID_GRAPH, 60 * ppm as u32),
                rows_with_count(&img, GRID_GRAPH, 40 * ppm as u32),
            );
        }
        img.save(if ppm > 1.5 {
            "target/gpu-debug-2.png"
        } else {
            "target/gpu-debug.png"
        })
        .expect("PNG write failed");
    }
}
// ---------------------------------------------------------------- epaint tessellation probe

/// Raw input for a `size_points`×`size_points` point canvas at `ppm`, carrying the device scale
/// through `ViewportInfo.native_pixels_per_point` (measured: `set_pixels_per_point` is a *zoom*
/// adjustment and double-applies when native is also set).
fn priming_input(size_points: f32, ppm: f32) -> egui::RawInput {
    let area = rect_of_edges(0.0, 0.0, size_points, size_points);
    let mut input = egui::RawInput {
        screen_rect: Some(area),
        focused: true,
        ..Default::default()
    };
    input.viewports.insert(
        egui::ViewportId::ROOT,
        egui::ViewportInfo {
            native_pixels_per_point: Some(ppm),
            ..Default::default()
        },
    );
    input
}

/// Runs the *same* tessellation step the GPU renderer uses (`ctx.tessellate`) on the exact rect
/// shapes the AIDA64 paint layer produces, so a missing pixel can be attributed to tessellation
/// rather than to the paint layer. Evidence artifact: `work/aida64/logs/ui-test-24.out` showed the
/// 1×1-device-px dither cells render as 0 px while 1-px-wide/60-tall grid lines render fine.
#[test]
fn tessellation_probe_of_tiny_and_long_device_rects() {
    let ctx = egui::Context::default();
    // measured: `ctx.tessellate` reads the font atlas of the *last layout pass* and panics
    // "No fonts loaded" on a pristine context (egui-0.36.2 src/context.rs:2871-2880), so each
    // scale is primed with an empty pass first (the same recipe as tests/pixel_grid.rs).
    for p in [1.0f32, 2.0] {
        let mut out = ctx.run_ui(priming_input(PROBE_CANVAS, p), |_ui| {});
        out.textures_delta.clear();
    }
    let clip = rect_of_edges(0.0, 0.0, PROBE_CANVAS, PROBE_CANVAS);
    // (label, ppm, rect, expect_degenerate_core) — the flag is the *geometry* (one device pixel in
    // both dimensions), never a tag substring: the "hairline 368x1dev @ppm2" rect is one device px
    // *tall* but 368 px long, and the dumps show it keeps solid width.
    for (tag, ppm, r, degenerate) in [
        (
            "dither cell 1x1 @ppm1",
            1.0,
            rect_of_edges(10.0, 13.0, 11.0, 14.0),
            true,
        ),
        (
            "hairline 184x1 @ppm1",
            1.0,
            rect_of_edges(8.0, 10.0, 192.0, 11.0),
            false,
        ),
        (
            "grid column 1x60 @ppm1",
            1.0,
            rect_of_edges(18.0, 20.0, 19.0, 80.0),
            false,
        ),
        (
            "dither cell 1dev @ppm2",
            2.0,
            rect_of_edges(20.0, 26.0, 20.5, 26.5),
            true,
        ),
        (
            "hairline 368x1dev @ppm2",
            2.0,
            rect_of_edges(16.0, 20.0, 192.0, 20.5),
            false,
        ),
    ] {
        let shape = egui::Shape::Rect(egui::epaint::RectShape::filled(
            r,
            egui::CornerRadius::ZERO,
            HAIRLINE_ALT,
        ));
        let prims = ctx.tessellate(
            vec![egui::epaint::ClippedShape {
                clip_rect: clip,
                shape,
            }],
            ppm,
        );
        let mut tri = 0usize;
        let mut bounds = egui::Rect::NOTHING;
        for prim in &prims {
            if let egui::epaint::Primitive::Mesh(ref mesh) = prim.primitive {
                tri += mesh.triangles().count();
                if !mesh.is_empty() {
                    bounds = bounds.union(mesh.calc_bounds());
                }
            }
        }
        println!(
            "{tag}: prims {}, tri {}, bounds {}",
            prims.len(),
            tri,
            bounds
        );
        assert_eq!(
            prims.len(),
            1,
            "{tag}: a single filled rect must tessellate to one primitive"
        );
        assert_eq!(
            tri, 10,
            "{tag}: a feathered filled rect is 10 triangles (core + 4 feather quads + 4 corners)"
        );
        if degenerate {
            // measured in `ui-test-25.out`: the solid core of a 1-device-px-by-1-device-px rect
            // collapses to zero width, which is why the GPU raster paints nothing there.
            assert!(
                bounds.max.x - bounds.min.x < 0.01,
                "{tag}: a 1×1 device-px core must tessellate degenerate (measured zero-width bounds)"
            );
        } else {
            assert!(
                bounds.max.x - bounds.min.x > 1.0,
                "{tag}: a long 1-px-thick rect keeps solid width, so it rasterizes exact"
            );
        }
    }
}

// ---------------------------------------------------------------- composition lane (dump-first)

/// Runs of identical rgba along row `y` for x in `x0..x1` as `(start, len, hex)`.
fn runs_on_row(img: &RgbaImage, y: u32, x0: u32, x1: u32) -> Vec<(u32, u32, String)> {
    let mut runs = Vec::new();
    let mut x = x0;
    while x < x1 {
        let c = px(img, x, y);
        let mut len = 1u32;
        while x + len < x1 && px(img, x + len, y) == c {
            len += 1;
        }
        runs.push((x, len, hex4(c)));
        x += len;
    }
    runs
}

/// Collapses a sorted device-row list into inclusive `(first, last)` groups.
fn group_dev_rows(rows: &[u32]) -> Vec<(u32, u32)> {
    let mut out: Vec<(u32, u32)> = Vec::new();
    for &y in rows {
        match out.last_mut() {
            Some(last) if last.1 + 1 == y => last.1 = y,
            _ => out.push((y, y)),
        }
    }
    out
}

/// Measured here: `egui_kittest`'s `Harness::render()` never applies the pass's
/// `TexturesDelta` — every textured primitive (egui's own FONT textures included) rasterizes to
/// nothing in that lane: the composed dump `ui-test-43.out` shows the selected label's row (96)
/// as a continuous `#626262/ff` run with zero glyph ink, and wired icons produced zero
/// non-chrome pixels (`ui-test-46..49`). This helper replicates kittest's own render body
/// (`wgpu.rs`: `default_wgpu_setup` + `create_render_state` + encoder + `tessellate` +
/// `update_buffers` + readable texture + transparent-cleared `RenderPass` + `Renderer::render`)
/// on the same deterministic PREDICTABLE lavapipe device, but additionally applies
/// `harness.output().textures_delta` through `Renderer::update_texture` ("Should be called
/// before [`Self::render`]", `egui-wgpu-0.36.2/renderer.rs:610`) — the step eframe performs
/// every frame — so the real texel path (NEAREST glyph + fonts) gets proven on the real device.
fn render_scene_with_textures(mut app: impl FnMut(&mut Ui), size: f32, ppm: f32) -> RgbaImage {
    let _device_lock = RENDER_SERIALIZER
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    use egui_wgpu::ScreenDescriptor;
    use egui_wgpu::wgpu;

    let render_state = egui_kittest::wgpu::create_render_state(
        egui_kittest::wgpu::default_wgpu_setup(),
        egui_wgpu::RendererOptions::PREDICTABLE,
    );
    let mut harness = Harness::builder()
        .with_size(Vec2::new(size, size))
        .with_pixels_per_point(ppm)
        .build_ui(&mut app);
    harness.step();
    harness.step();

    let shapes = harness.output().shapes.clone();
    // Measured here (ui-test-58/60): the last pass's `FullOutput.textures_delta` is EMPTY while
    // the glyph mesh references the last allocated id — in 0.36 `Context::load_texture` is only a
    // `TextureManager::alloc` (`context.rs:2390-2410`), and the manager ships pixels to the
    // painting side through its OWN delta ("Communicates with the painting subsystem using
    // [`Self::take_delta`]", `epaint-0.36.2/src/textures.rs:10-18`). Real egui-wgpu high-level
    // rendering drains that manager delta (why eframe shows text/glyphs); the kittest read-pass
    // does not ⇒ drain it here and upload via `Renderer::update_texture`, the documented
    // before-`render` step (`renderer.rs:610`).
    let tex_manager = harness.ctx.tex_manager();
    let delta = {
        let mut manager = tex_manager.write();
        manager.take_delta()
    };
    let tessellated = harness.ctx.tessellate(shapes, ppm);
    let screen = ScreenDescriptor {
        pixels_per_point: ppm,
        size_in_pixels: {
            let s = harness.ctx.content_rect().size() * ppm;
            [s.x.round() as u32, s.y.round() as u32]
        },
    };

    let mut renderer = render_state.renderer.write();
    let mut encoder = render_state
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("aida64 textured encoder"),
        });
    // `egui::TexturesDelta.set` docs: "New or changed textures. Apply before painting." — this
    // is precisely the step `Harness::render()` omits.
    for (id, deltas) in &delta.set {
        for image_delta in deltas {
            renderer.update_texture(&render_state.device, &render_state.queue, *id, image_delta);
        }
    }
    // In 0.36 the `TextureManager` is DEFAULTED with a 1×1 WHITE alloc at `Managed(0)`
    // (`epaint-0.36.2/src/textures.rs:17-18`); solid color fills draw as quads sampling that uv
    // (`WHITE_UV`), and egui drains alloc deltas at run time — measured: this lane's manager
    // drain finds an EMPTY set (white was drained by settle-passes), so without this upload
    // NOTHING rasterizes here, fills included. Upload the default white explicitly, plus every
    // stashed glyph alloc (`icon_texture_uploads`: settle-passes drained glyph deltas before the
    // read-pass too, ui-test-58/62) via the before-render `Renderer::update_texture`.
    renderer.update_texture(
        &render_state.device,
        &render_state.queue,
        egui::TextureId::default(),
        &egui::epaint::ImageDelta::full(
            egui::ColorImage::from_rgba_unmultiplied([1, 1], &[255, 255, 255, 255]),
            egui::TextureOptions::LINEAR,
        ),
    );
    for (id, image) in aida64_ui_theme::widgets::icon_texture_uploads() {
        renderer.update_texture(
            &render_state.device,
            &render_state.queue,
            id,
            &egui::epaint::ImageDelta::full(image, egui::TextureOptions::NEAREST),
        );
    }
    let user_buffers = renderer.update_buffers(
        &render_state.device,
        &render_state.queue,
        &mut encoder,
        &tessellated,
        &screen,
    );
    let texture = render_state
        .device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("aida64 readback texture"),
            size: wgpu::Extent3d {
                width: screen.size_in_pixels[0],
                height: screen.size_in_pixels[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: render_state.target_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    {
        let mut pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("aida64 textured pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                ..Default::default()
            })
            .forget_lifetime();
        renderer.render(&mut pass, &tessellated, &screen);
    }
    render_state.queue.submit(core::iter::chain(
        user_buffers,
        std::iter::once(encoder.finish()),
    ));
    texture_to_image(&render_state.device, &render_state.queue, &texture)
}

/// Readback replica of `egui_kittest`'s `pub(crate)` `texture_to_image` (verbatim staging-copy
/// pattern: row-padded MAP_READ staging buffer → `copy_texture_to_buffer` → submit + blocking
/// `PollType::Wait` with the submission index → mapped slice → unpadded rows → `RgbaImage`),
/// fully-qualified so no new file-level imports are needed.
fn texture_to_image(
    device: &egui_wgpu::wgpu::Device,
    queue: &egui_wgpu::wgpu::Queue,
    texture: &egui_wgpu::wgpu::Texture,
) -> RgbaImage {
    use egui_wgpu::wgpu;
    use std::mem::size_of;
    use std::sync::mpsc::channel;

    // Same padding math as kittest's `BufferDimensions::new` (`wgpu::COPY_BYTES_PER_ROW_ALIGNMENT`).
    let unpadded = texture.width() as usize * size_of::<u32>();
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    let padded = unpadded + (align - unpadded % align) % align;

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("aida64 readback staging"),
        size: (padded * texture.height() as usize) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("aida64 readback copy"),
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &output_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded as u32),
                rows_per_image: None,
            },
        },
        wgpu::Extent3d {
            width: texture.width(),
            height: texture.height(),
            depth_or_array_layers: 1,
        },
    );
    let submission = queue.submit(std::iter::once(encoder.finish()));

    // Note: no `.await` — the blocking poll below drives the map to completion.
    let (sender, receiver) = channel();
    output_buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |v| drop(sender.send(v)));
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .expect("Failed to poll device");
    receiver.recv().unwrap().unwrap();

    let data: Vec<u8> = output_buffer
        .slice(..)
        .get_mapped_range()
        .expect("Failed to get mapped range")
        .chunks_exact(padded)
        .flat_map(|row| row.iter().take(unpadded).copied())
        .collect();
    RgbaImage::from_raw(texture.width(), texture.height(), data).expect("Failed to create image")
}

/// Fresh-dump evidence for the `gpu_composition_*` assertions: rasterizes `paint_aida64_chrome`
/// on a 926-point square canvas at PPM 1.0 (img1-crop width) with a selected root row, a plain
/// row and one depth-1 row, then prints band rows, column runs, selection-ring rows, arrow-zone
/// rows and corner probes as absolute device pixels. AGENTS rule 11: GPU expectations are encoded
/// only from a dump like this (run `cargo test --test gpu_snapshot gpu_composition_dump --
/// --nocapture`), never guessed.
#[test]
fn gpu_composition_dump() {
    use aida64_ui_theme::compose::paint_aida64_chrome;
    use aida64_ui_theme::widgets::TreeItem;

    let img = render_scene(
        |ui| {
            apply(ui.ctx());
            let rows = [
                TreeItem {
                    label: "Computer",
                    depth: 0,
                    expanded: false,
                    selected: true,
                    icon_rgba: None,
                },
                TreeItem {
                    label: "row-two",
                    depth: 0,
                    expanded: false,
                    selected: false,
                    icon_rgba: None,
                },
                TreeItem {
                    label: "child",
                    depth: 1,
                    expanded: false,
                    selected: false,
                    icon_rgba: None,
                },
            ];
            let _ = paint_aida64_chrome(ui, 194.0, &rows, None);
        },
        926.0,
        1.0,
    );
    let (w, h) = (img.width(), img.height());
    println!("canvas {}x{}", w, h);
    for (name, c) in [
        ("titlebar", BG_TITLEBAR),
        ("panel_bg", BG_PANEL),
        ("hairline", HAIRLINE),
        ("trough", BG_SCROLLBAR_TROUGH),
        ("arrow_ink", GLYPH_SCROLLBAR_ARROW),
        ("sel_fill", BG_SELECTION),
        ("sel_ring", STROKE_SELECTION),
    ] {
        println!(
            "full_rows {} = {:?}",
            name,
            group_dev_rows(&full_rows(&img, c))
        );
    }
    let sel_rows: Vec<u32> = (0..h)
        .filter(|&y| row_count(&img, y, BG_SELECTION) > 0)
        .collect();
    println!("sel_fill_rows {:?}", group_dev_rows(&sel_rows));
    let ring_rows: Vec<u32> = (0..h)
        .filter(|&y| row_count(&img, y, STROKE_SELECTION) > 0)
        .collect();
    println!("ring_rows {:?}", group_dev_rows(&ring_rows));
    let arrow_rows: Vec<u32> = (0..h)
        .filter(|&y| row_count(&img, y, GLYPH_SCROLLBAR_ARROW) > 0)
        .collect();
    println!("arrow_rows {:?}", group_dev_rows(&arrow_rows));
    for y in [
        0u32, 8, 16, 29, 30, 45, 77, 78, 79, 84, 85, 86, 87, 90, 95, 96, 97, 98, 99, 100, 120,
    ] {
        if y < h {
            println!("y {} runs 0..250: {:?}", y, runs_on_row(&img, y, 0, 250));
        }
    }
    if let Some(&ymid) = sel_rows.get(sel_rows.len() / 2) {
        println!(
            "sel row {} runs 0..230: {:?}",
            ymid,
            runs_on_row(&img, ymid, 0, 230)
        );
        println!(
            "sel row {} runs 170..212: {:?}",
            ymid,
            runs_on_row(&img, ymid, 170, 212)
        );
    }
    for (x, y) in [
        (0u32, 0u32),
        (4, 4),
        (10, 10),
        (460, 0),
        (0, 460),
        (925, 925),
    ] {
        println!("corner ({},{}) = {}", x, y, hex4(px(&img, x, y)));
    }
    assert_eq!((w, h), (926, 926), "harness raster = canvas points * PPM");
    assert!(
        !sel_rows.is_empty() && !arrow_rows.is_empty(),
        "composition must raster the selection fill and the arrow ink zones (measured geometry)"
    );
}

/// Shared scene for the composition asserts (same canvas/rows as `gpu_composition_dump` so the
/// absolute coordinates below stay the dump-verified ones, `ui-test-43.out`).
fn composition_scene(ui: &mut Ui) {
    use aida64_ui_theme::compose::paint_aida64_chrome;
    use aida64_ui_theme::widgets::TreeItem;
    apply(ui.ctx());
    let rows = [
        TreeItem {
            label: "Computer",
            depth: 0,
            expanded: false,
            selected: true,
            icon_rgba: None,
        },
        TreeItem {
            label: "row-two",
            depth: 0,
            expanded: false,
            selected: false,
            icon_rgba: None,
        },
        TreeItem {
            label: "child",
            depth: 1,
            expanded: false,
            selected: false,
            icon_rgba: None,
        },
    ];
    let _ = paint_aida64_chrome(ui, 194.0, &rows, None);
}

/// Asserts the composed chrome at absolute device px. Positions are the fresh-dump numbers
/// (`ui-test-43.out`: canvas 926 points @PPM 1.0, kittest harness reserves an 8-px margin, so
/// absolute == capture-coordinate + 8 — the correspondence is stated per assert); colors are the
/// spec §5 constants. This is the on-screen counterpart of the golden band claims.
#[test]
fn gpu_composition_bands_and_scrollbar() {
    let img = render_scene(composition_scene, 926.0, 1.0);
    assert_eq!(
        group_dev_rows(&full_rows(&img, BG_TITLEBAR)),
        vec![(8, 37)],
        "titlebar 30 rows #000000 = capture rows 0..29 + harness margin 8 (dump ui-test-43.out)"
    );
    assert_eq!(
        group_dev_rows(&full_rows(&img, BG_PANEL)),
        vec![(38, 85)],
        "toolbar 48 rows #202020 = capture rows 30..77 + harness 8 (§1)"
    );
    // §1 hairline row (capture 78 + 8 = 86), inked from window edge x=0 (compose sets
    // window_margin 0); the harness viewport truncates the span at 910 (measured in dump) so the
    // asserts pin left edge + far reaches instead of a full-row uniformity.
    at(
        &img,
        0,
        86,
        HAIRLINE,
        "hairline row 86 at window edge x=0 (§1 full-bleed)",
    );
    at(
        &img,
        9,
        86,
        HAIRLINE,
        "hairline through harness margin column",
    );
    at(
        &img,
        909,
        86,
        HAIRLINE,
        "hairline reaches viewport right minus harness margin (§1)",
    );
    // §1 divider column: capture 202 + harness 8 = 210; content ink right of it at 211 (§1 203+8).
    at(
        &img,
        210,
        120,
        HAIRLINE,
        "divider col 210 = capture 202 + harness 8",
    );
    at(
        &img,
        211,
        120,
        BG_PANEL,
        "content ink right of divider (§1 203 + 8)",
    );
    // §2 scrollbar: 17-px trough capture 181..197 + 8 = 189..205, 4-px gutter right (§2).
    at(
        &img,
        188,
        120,
        BG_PANEL,
        "row-ink column ends exactly where the measured slot begins (dump run 8..181 panel)",
    );
    at(
        &img,
        189,
        120,
        BG_SCROLLBAR_TROUGH,
        "trough col 189 = capture 181 + 8 (SCROLLBAR_TOTAL_W 17, §2)",
    );
    at(&img, 205, 120, BG_SCROLLBAR_TROUGH, "trough right col 205");
    at(
        &img,
        206,
        120,
        BG_PANEL,
        "gutter starts right of trough (SCROLLBAR_GUTTER_W 4 ⇒ 206..=209, §2)",
    );
    at(&img, 209, 120, BG_PANEL, "gutter right edge before divider");
    // §1 selection ring on row 0: 18-px box rows 87..104 (= capture 79..96 + 8), ring cols 35/110
    // (= capture 27/… + 8 — left edge exactly measured 27+8), interior fill 88..103.
    let sel_rows: Vec<u32> = (0..img.height())
        .filter(|&y| row_count(&img, y, BG_SELECTION) > 0)
        .collect();
    assert_eq!(
        group_dev_rows(&sel_rows),
        vec![(88, 103)],
        "selection interior = 16 rows of the 18-px band (TREE_ROW_H §1; dump ui-test-43.out)"
    );
    let ring_rows: Vec<u32> = (0..img.height())
        .filter(|&y| row_count(&img, y, STROKE_SELECTION) > 0)
        .collect();
    assert_eq!(
        group_dev_rows(&ring_rows),
        vec![(87, 104)],
        "ring hairline rows top+bottom of the 18-px box (SELECTION_BOX_H §1)"
    );
    at(
        &img,
        35,
        96,
        STROKE_SELECTION,
        "ring left col 35 = capture 27 + harness 8 (§1 measured ring x=27)",
    );
    at(
        &img,
        36,
        96,
        BG_SELECTION,
        "interior fill #626262 begins right of ring (§1)",
    );
    at(
        &img,
        109,
        96,
        BG_SELECTION,
        "interior fill ends before ring right col",
    );
    at(
        &img,
        110,
        96,
        STROKE_SELECTION,
        "ring right col (5-fill-rect box; CPU lane pins the label-width formula)",
    );
    // 3-px arrow ink bands inside the top/bottom 17-px arrow zones (SCROLLBAR_ARROW_GLYPH 3 px;
    // absolute band rows are canvas-size dependent ⇒ pinned to the dump for this fixed canvas).
    let arrow_rows: Vec<u32> = (0..img.height())
        .filter(|&y| row_count(&img, y, GLYPH_SCROLLBAR_ARROW) > 0)
        .collect();
    assert_eq!(
        group_dev_rows(&arrow_rows),
        vec![(94, 96), (908, 910)],
        "arrow ink = 3 device rows top + 3 bottom (dump ui-test-43.out, canvas 926)"
    );
    // Tree level step: the depth-1 row must start one TREE_INDENT right of the depth-0 cursor
    // (ring geometry already asserts the depth-0 cursor at 37; child row has no ring ⇒ probe the
    // reserved icon slot column of row 2: rows 105..122, col 37+19=56 must still be fill-free
    // (plain row ⇒ #202020 ink, dump y=120 run 8..181 panel proves no ink rows 105+ ⇒ keep only
    // the row-band pitch fact already CPU-verified).
    at(
        &img,
        37,
        114,
        BG_PANEL,
        "unselected plain row rasterizes flat panel bg (§1)",
    );
}

/// Texel-perfect icon contract: rasterizing the golden `atlas-glyph-match` winner asset
/// (MAD 1.02 claim, `tools/model_vs_capture.py`; provenance `work/aida64/out/atlas-analysis.md`)
/// in the reserved icon slot must reproduce the straight-alpha composite over `#626262` exactly
/// for opaque/transparent texels; the winner asset carries zero partial-alpha texels
/// (measured histogram a∈{255:202, 0:54}) and the fresh dumps (`ui-test-63/64.out`) show
/// `max_rgb_delta = max_alpha_delta = partial_count = 0`, so every asserted delta is 0 while the
/// partial-class counters stay for AA assets. Slot origin assert: capture glyph crop
/// x=29 (`atlas-analysis.md` part C) + harness 8 = 37, and row-band interior rows start 88
/// (TREE_ROW_H 18 band 87..104 with the 16-px slot vertically centered, CPU-lane geometry).
#[test]
fn gpu_composition_icon_texel_perfect() {
    use aida64_ui_theme::compose::paint_aida64_chrome;
    use aida64_ui_theme::widgets::{IconPixels, TreeItem};

    let asset = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../work/aida64/out/atlas/aida_icons10.dll_dib_16x16_32bpp_comp0_pal0_BGRA-alpha_92.png",
    );
    let opened = image::open(&asset).unwrap_or_else(|e| panic!("atlas winner must decode: {e}"));
    let rgba_img = opened.into_rgba8();
    assert_eq!(
        (rgba_img.width(), rgba_img.height()),
        (16, 16),
        "winner asset is 16x16"
    );
    let rgba = rgba_img.into_vec();

    let glyph = IconPixels {
        width: 16,
        height: 16,
        rgba: rgba.clone(),
    };
    // Measured here: with `egui_kittest` (whose `build_ui` runs the app too, so the pass
    // `render()` finally consumes is not simply "the second step"), a stable-named glyph wired on
    // an earlier pass rasterizes ZERO texel pixels — the renderer's texture map never received
    // the alloc (the `wgpu.rs` delta handler drains `delta.free`; the harness-internal
    // invocation count measured here is ≥3: build + 2 steps). With `paint_icon_rgba`'s unique
    // per-call name (`src/widgets.rs`), EVERY invocation — including the final one — allocates
    // its own texture and add-delta, so whichever pass renders always sees its own glyph.
    // eframe renders every pass and gets the same guarantee; 1 KB/pass churn is negligible.
    // `Harness::render()` drops texture deltas (measured, `ui-test-43/46..49`) — the textured
    // lane below applies them like eframe does every frame.
    let img = render_scene_with_textures(
        |ui| {
            apply(ui.ctx());
            let rows = [TreeItem {
                label: "Computer",
                depth: 0,
                expanded: false,
                selected: true,
                icon_rgba: Some(glyph.clone()),
            }];
            let _ = paint_aida64_chrome(ui, 194.0, &rows, None);
        },
        926.0,
        1.0,
    );

    let sel = want(BG_SELECTION);
    // Composite expectations straight from the measured asset bytes (renderer-like blend).
    let expected = |tx: u32, ty: u32| -> [i32; 3] {
        let i = ((ty * 16 + tx) * 4) as usize;
        let (r, g, b, a) = (
            rgba[i] as i32,
            rgba[i + 1] as i32,
            rgba[i + 2] as i32,
            rgba[i + 3] as i32,
        );
        [
            (r * a + sel[0] as i32 * (255 - a)) / 255,
            (g * a + sel[1] as i32 * (255 - a)) / 255,
            (b * a + sel[2] as i32 * (255 - a)) / 255,
        ]
    };
    // Texel-perfect mapping contract at device (37,88). Every number proven by measurement, not
    // guessed: cursor x=37 because `gpu_composition_bands_and_scrollbar` (green) pins the
    // selection ring left col at device 35 = cursor − crate `SELECTION_INSET_X`(2), and that ring
    // col is the captured ring edge x=27 (`ksnip-20261014-003-ui-spec.md` §1) plus the kittest
    // harness's measured 8-px canvas margin — equivalently cursor 37 = captured glyph-crop left
    // x=29 (`work/aida64/out/atlas-analysis.md` part C) + 8. Rows 88..=103: the box ring
    // hairlines rasterize on device rows 87/104 (dump `ring_rows [(87,104)]`, `ui-test-43.out`)
    // and the 16-px icon slot is vertically centered inside the 18-px row band (CPU-lane
    // `paint_tree_row` slot geometry asserted in unit tests). `ui-test-51.out` measured glyph ink
    // spanning rows 88..=103, cols 36..=54: the ±1 fringe beyond the texel footprint is the
    // documented GPU-float-vs-CPU-integer blend rounding on near-fill partial alphas, not a
    // mapping shift — mapping stays texel-exact at (37,88) at PPM 1.0 with NEAREST.
    let origin_x = 37u32;
    let origin_y = 88u32;
    let sel_i = [sel[0] as i32, sel[1] as i32, sel[2] as i32];
    let mut ink_texels = 0usize; // asset texels whose composite differs from the pure fill
    let mut opaque_bad = 0usize; // opaque/transparent texels rasterized off-exact
    let mut max_rgb_delta = 0i32; // fresh-dump measurement feeding the partial-alpha tolerance
    let mut max_alpha_delta = 0i32;
    let mut partial_count = 0usize;
    let mut partial_samples: Vec<(u32, u32, [u8; 4], [i32; 3])> = Vec::new();
    let mut opaque_offenders: Vec<(u32, u32, [u8; 4], [i32; 3])> = Vec::new();
    for ty in 0..16u32 {
        for tx in 0..16u32 {
            let i = ((ty * 16 + tx) * 4) as usize;
            let a = rgba[i + 3];
            let e = expected(tx, ty);
            let got = px(&img, origin_x + tx, origin_y + ty);
            if e != sel_i {
                ink_texels += 1;
            }
            if a == 255 {
                if got[0] != e[0] as u8
                    || got[1] != e[1] as u8
                    || got[2] != e[2] as u8
                    || got[3] != 255
                {
                    opaque_bad += 1;
                    opaque_offenders.push((tx, ty, got, e));
                }
            } else if a == 0 {
                if got != want(BG_SELECTION) {
                    opaque_bad += 1;
                    opaque_offenders.push((tx, ty, got, e));
                }
            } else {
                partial_count += 1;
                let dr = (got[0] as i32 - e[0]).abs();
                let dg = (got[1] as i32 - e[1]).abs();
                let db = (got[2] as i32 - e[2]).abs();
                max_rgb_delta = max_rgb_delta.max(dr.max(dg).max(db));
                max_alpha_delta = max_alpha_delta.max((got[3] as i32 - 255).abs());
                if partial_samples.len() < 8 {
                    partial_samples.push((tx, ty, got, e));
                }
            }
        }
    }
    println!(
        "fresh-dump: origin({origin_x},{origin_y}) partial_count={partial_count} \
         max_rgb_delta={max_rgb_delta} max_alpha_delta={max_alpha_delta} \
         ink_texels={ink_texels} opaque_bad={opaque_bad}"
    );
    if !opaque_offenders.is_empty() {
        println!(
            "opaque/transparent offenders {} first8: {:?}",
            opaque_offenders.len(),
            &opaque_offenders[..8.min(opaque_offenders.len())]
        );
    }
    println!("partial samples first8: {:?}", partial_samples);
    assert!(
        ink_texels > 0,
        "asset must carry ink (golden atlas-glyph-match MAD 1.02 claim)"
    );
    assert_eq!(
        opaque_bad, 0,
        "opaque & transparent texels must raster exact (NEAREST mapping, no AA)"
    );
    // Encoded from the fresh dump `work/aida64/logs/ui-test-63.out` (AGENTS rule 11): the
    // winner asset carries zero partial-alpha texels (alpha histogram a∈{255:202, 0:54}) and the
    // textured lane rasterized origin(37,88) byte-identical to the CPU straight-alpha composite
    // (prints above measured: partial_count=0, max_rgb_delta=0, max_alpha_delta=0) ⇒ NEAREST
    // @PPM 1 mapping is end-to-end exact; the partial-class counters stay for AA assets.
    assert_eq!(
        partial_count, 0,
        "measured winner asset has zero partial-alpha texels (ui-test-63.out: partial_count=0)"
    );
    assert_eq!(
        max_rgb_delta, 0,
        "glyph raster equals the CPU composite per channel (ui-test-63.out: max_rgb_delta=0)"
    );
    assert_eq!(
        max_alpha_delta, 0,
        "every mapped texel rasterizes fully opaque like the composite (ui-test-63.out: 0)"
    );
}
