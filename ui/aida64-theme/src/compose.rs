//! Composes the measured AIDA64 chrome bands from the crate primitives.
//!
//! Every geometry/color here is a measured value from
//! `work/aida64/out/ksnip-20261014-003-ui-spec.md` (PPM 1.0, calibration recorded in
//! `metrics::CALIBRATION_PPM`):
//! - §1: titlebar 30 px `#000000` inked edge-to-edge (x=0..925); toolbar 48 px `#202020`
//!   ending at row 77; 1-px hairline row 78 (`paint::paint_hairline_h`, HAIRLINE — the dither
//!   alternation ×370 vs ×938 is unverified, so the single-line variant is painted, per rule 11);
//!   tree panel ink x=8..201 (width 194 = `metrics::TREE_PANEL_W_CAPTURE`, a *capture crop*
//!   instance ⇒ caller-configurable), divider column 202 (1-px `paint_hairline_v`);
//!   tree content left x=29 (selected-row ring left 27 minus `SELECTION_INSET_X` 2), panel ink
//!   left 8 ⇒ inner left pad 21 (`metrics::TREE_PANEL_INNER_LEFT`).
//! - §2/§3: scrollbar 17 px slot (`SCROLLBAR_TOTAL_W`) + 4 px gutter (`SCROLLBAR_GUTTER_W`)
//!   painted inside the tree panel's right edge via `paint::paint_aida64_scrollbar`; the absolute
//!   slot placement is a layout choice (capture column x is asserted only by the golden tool on
//!   the captures themselves), the widths/colors are measured.
//! - Selection/text: rows are `widgets::TreeItem` — 18 px rows, `Ui::indent` step
//!   `metrics::TREE_INDENT` 19 (spacing patched by `style::aida64_style`), 5-rect custom
//!   selection ring + `#FFFFFF` ink bypassing egui's `SELECTED_CLASS` text recolor.
//!
//! Full-bleed vs inset: the capture titlebar/toolbar are inked from window edge x=0 while the
//! tree panel ink starts at x=8 ⇒ this module sets `spacing.window_margin = 0` on its own style
//! copy and encodes the 8-px content inset in the tree panel's *frame outer margin* (measured
//! provenance above, not egui defaults — egui's `window_margin` default is `Margin::same(6)`,
//! `egui-0.36.2/src/style.rs:1456`, and `Panel::exact_size(f32)` sizes **along the axis**
//! incl. margins, `containers/panel.rs:405`; `InnerResponse.response.rect` is the shifted outer
//! rect, `panel.rs:848-849` — all facts used below).
//!
//! Central-panel inner padding and the ribbon/tabs/log-list are NOT composed yet (tile x-offset
//! unmeasured ⇒ documented open, no fabricated ink).

use egui::{CentralPanel, Margin, Panel, Rect, Ui};

use crate::metrics;
use crate::paint;
use crate::palette;
use crate::style;
use crate::widgets::TreeItem;

/// Painted ink rects of the bands (frame margins removed ⇒ measured capture ink, e.g. tree
/// cols 8..202 exclusive; titlebar/toolbar margins are zero so their rects are full-bleed 0..w)., in UI points (PPM from the context). Returned so callers
/// (and the harness tests) can assert device-pixel geometry without re-deriving margins.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChromeRects {
    pub titlebar: Rect,
    pub toolbar: Rect,
    pub tree: Rect,
    pub central: Rect,
    /// y of the 1-px toolbar→content hairline row (its own rect row).
    pub hairline_y: f32,
}

/// Paints the chrome onto `ui` (assumes `style::apply` ran on the same context).
///
/// `tree_width_points` is the tree panel measured INK width (outer rect = INK + the 8-px inset, since `exact_size` reserves incl. margins — panel.rs:405) — the capture instance is
/// `metrics::TREE_PANEL_W_CAPTURE` 194 but that is a *crop* artifact ⇒ parameterized (rule 11:
/// do not treat crop instances as gospel). `thumb_rows` optionally marks the thumb row span
/// (row-index units, *illustrative* extent — captured thumb extent was not measured ⇒ passing
/// `None` paints only trough + arrows, all measured).
pub fn paint_aida64_chrome(
    ui: &mut Ui,
    tree_width_points: f32,
    rows: &[TreeItem],
    thumb_rows: Option<(usize, usize)>,
) -> ChromeRects {
    // The capture ink geometry (see module docs): window margin 0 for the edge-to-edge top bands;
    // the 8-px left content inset lives in the tree panel frame margin instead. The row-indent
    // step is re-patched here from the measurement (spec §1): a harness can deliver a fresh
    // default spacing per pass (measured: the kittest GPU harness reset style, so relying on a
    // prior `apply()` alone tripped the guard in that lane).
    {
        let st = ui.style_mut();
        st.spacing.window_margin = Margin::ZERO;
        st.spacing.indent = metrics::TREE_INDENT;
    }
    debug_assert!(
        crate::widgets::indent_step_is_measured(ui.style().spacing.indent),
        "spacing.indent must equal TREE_INDENT when composing rows"
    );

    let full_w = ui.available_width();

    let mut tb_frame = style::titlebar_frame();
    tb_frame.outer_margin = Margin::ZERO;
    tb_frame.inner_margin = Margin::ZERO;
    let titlebar = Panel::top("aida64_titlebar")
        .exact_size(metrics::TITLEBAR_H)
        .frame(tb_frame)
        .show_separator_line(false)
        .show(ui, |_ui| {});

    let mut tb2_frame = style::toolbar_frame();
    tb2_frame.outer_margin = Margin::ZERO;
    tb2_frame.inner_margin = Margin::ZERO;
    let toolbar = Panel::top("aida64_toolbar")
        .exact_size(metrics::TOOLBAR_H)
        .frame(tb2_frame)
        .show_separator_line(false)
        .show(ui, |_ui| {});

    // Measured band boundary: toolbar ink rows …77, hairline row 78 (§1). Painted explicitly
    // (crisp fill-rect, not the feathered Separator line) across the full canvas width.
    let hairline_y = toolbar.response.rect.max.y;
    paint::paint_hairline_h(ui.painter(), hairline_y, 0.0, full_w, palette::HAIRLINE);

    let mut tree_frame = style::panel_frame();
    tree_frame.outer_margin = Margin {
        left: metrics::TREE_PANEL_LEFT_INSET as i8,
        right: 0,
        top: 1, // measured hairline row 78 is chrome ink ⇒ tree ink must start at row 79 (§1)
        bottom: 0,
    };
    tree_frame.inner_margin = Margin {
        left: metrics::TREE_PANEL_INNER_LEFT as i8,
        right: 0,
        top: 0,
        bottom: 0,
    };
    let tree = Panel::left("aida64_tree")
        // `exact_size` sizes the reservation **including margins** (measured panel.rs:405), so
        // reserving INK + the 8-px inset yields outer rect [0..202] and frame ink [8..202] —
        // the captured 194-px band x=8..201 with the divider column exactly at 202 (§1).
        .exact_size(tree_width_points + metrics::TREE_PANEL_LEFT_INSET)
        .frame(tree_frame)
        .show_separator_line(false)
        .show(ui, |ui| {
            // 17-px slot + 4-px gutter reserved from the measured widths on the panel's right
            // side; the row column keeps everything else (tile/row x-offsets beyond that are
            // layout choices, asserted internally by the harness tests).
            let panel_w = tree_width_points; // INK width
            let slot_w = metrics::SCROLLBAR_TOTAL_W + metrics::SCROLLBAR_GUTTER_W;
            // Content region inside the panel = INK − inner pad (21) = 173 px; reserving the
            // measured scrollbar column (21 px) leaves 152 px of row ink (§2 widths).
            ui.set_max_width(panel_w - metrics::TREE_PANEL_INNER_LEFT - slot_w);
            for (i, row) in rows.iter().enumerate() {
                let id = ui.id().with("aida64_tree_row").with(i);
                // Ui::indent walks exactly `spacing.indent` = TREE_INDENT 19 px (§1); nest
                // exactly `depth` times so each level steps exactly one measured indent.
                add_nested(ui, id, row.depth, row);
            }
        });

    // Reservation minus frame margins = the painted INK rect (§1: cols 8..201 inclusive ⇒
    // exclusive right 202; rows 79.. because hairline row 78 is chrome ink itself).
    let tree_ink = paint::rect_of_edges(
        tree.response.rect.min.x + metrics::TREE_PANEL_LEFT_INSET,
        tree.response.rect.min.y + 1.0,
        tree.response.rect.max.x,
        tree.response.rect.max.y,
    );
    // 1-px divider column just right of the tree ink (§1: ink …201, divider 202).
    paint::paint_hairline_v(
        ui.painter(),
        tree_ink.max.x,
        tree_ink.min.y,
        tree_ink.max.y,
        palette::HAIRLINE,
    );

    // Scrollbar: trough/arrows/thumb composed from measured pieces inside the tree panel's right
    // edge (slot geometry measured; thumb *extent* unmeasured ⇒ Option documented above).
    {
        let p = ui.painter();
        let slot_right = tree_ink.max.x - metrics::SCROLLBAR_GUTTER_W;
        let slot = paint::snap_rect(
            Rect::from_min_max(
                egui::pos2(slot_right - metrics::SCROLLBAR_TOTAL_W, tree_ink.min.y),
                egui::pos2(slot_right, tree_ink.max.y),
            ),
            ui.pixels_per_point(),
        );
        let arrows_top = Rect::from_min_max(
            slot.min,
            egui::pos2(slot.max.x, slot.min.y + metrics::SCROLLBAR_ARROW_ZONE),
        );
        let arrows_bottom = Rect::from_min_max(
            egui::pos2(slot.min.x, slot.max.y - metrics::SCROLLBAR_ARROW_ZONE),
            slot.max,
        );
        match thumb_rows {
            Some((t, b)) => {
                let top = slot.min.y + (t as f32) * metrics::TREE_ROW_H;
                let bot = slot.min.y + (b as f32) * metrics::TREE_ROW_H;
                let thumb = paint::snap_rect(
                    Rect::from_min_max(egui::pos2(slot.min.x, top), egui::pos2(slot.max.x, bot)),
                    ui.pixels_per_point(),
                );
                let _ = crate::widgets::paint_aida64_scrollbar(p, slot, thumb, true);
            }
            None => {
                // Thumb extent is unmeasured ⇒ trough + arrow zones only (no fabricated thumb).
                let _ = paint::paint_scrollbar_slot(p, slot);
                let _ = paint::paint_scrollbar_arrow_ink(p, arrows_top);
                let _ = paint::paint_scrollbar_arrow_ink(p, arrows_bottom);
            }
        }
    }

    // The captured divider column (202) and hairline row (78) are ink themselves ⇒ content must
    // start at x=203, y=79 (§1) ⇒ 1-px outer margins on the central frame.
    let mut central_frame = style::panel_frame();
    central_frame.outer_margin = Margin {
        left: 1,
        right: 0,
        top: 1,
        bottom: 0,
    };
    let central = CentralPanel::default()
        .frame(central_frame)
        .show(ui, |_ui| {});

    // Central ink = reservation minus the 1-px divider/hairline margins (§1: content 203/79).
    let central_ink = paint::rect_of_edges(
        central.response.rect.min.x + 1.0,
        central.response.rect.min.y + 1.0,
        central.response.rect.max.x,
        central.response.rect.max.y,
    );

    ChromeRects {
        titlebar: titlebar.response.rect,
        toolbar: toolbar.response.rect,
        tree: tree_ink,
        central: central_ink,
        hairline_y,
    }
}

/// Nest a row exactly `remaining` indent levels deep (spec §1: 19 px per level via
/// `Ui::indent`; `Ui::indent` returns the row's response, not a nested `Ui`, so recursion is the
/// only way to get exactly N steps without borrowing gymnastics).
fn add_nested(ui: &mut Ui, id: egui::Id, remaining: usize, row: &TreeItem) {
    if remaining == 0 {
        let _ = ui.add(row.clone());
    } else {
        ui.indent(id.with("lvl"), |ui| add_nested(ui, id, remaining - 1, row));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{RawInput, ViewportId, ViewportInfo};

    fn priming(size: f32) -> RawInput {
        let mut input = RawInput {
            screen_rect: Some(Rect::from_min_max(
                egui::pos2(0.0, 0.0),
                egui::pos2(size, size),
            )),
            focused: true,
            ..Default::default()
        };
        input.viewports.insert(
            ViewportId::ROOT,
            ViewportInfo {
                native_pixels_per_point: Some(1.0),
                ..Default::default()
            },
        );
        input
    }

    /// Runs one layout pass on a 926×668 canvas (img1 crop size) and checks the band outer rects
    /// against the capture geometry (PPM 1, crisp ⇒ every band edge must land on an integer).
    #[test]
    fn chrome_rects_match_capture_geometry() {
        let ctx = egui::Context::default();
        style::apply(&ctx);
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
        ];
        let mut rects_opt: Option<ChromeRects> = None;
        let mut priming_out = ctx.run_ui(priming(926.0), |_ui| {});
        priming_out.textures_delta.clear(); // epaint textures.rs:337 drop panic guard
        let mut out = ctx.run_ui(priming(926.0), |ui| {
            rects_opt = Some(paint_aida64_chrome(ui, 194.0, &rows, None));
        });
        out.textures_delta.clear();
        let r = rects_opt.expect("chrome painted");

        // §1 measured band heights, full-bleed from x=0.
        assert_eq!(r.titlebar.min.x, 0.0);
        assert_eq!(r.titlebar.max.x, 926.0);
        assert_eq!(r.titlebar.max.y - r.titlebar.min.y, metrics::TITLEBAR_H);
        assert_eq!(r.toolbar.min.y, metrics::TITLEBAR_H);
        assert_eq!(r.toolbar.max.y - r.toolbar.min.y, metrics::TOOLBAR_H);
        assert_eq!(r.hairline_y, metrics::TITLEBAR_H + metrics::TOOLBAR_H); // row 78

        // ChromeRects returns PAINTED INK rects (frame margins subtracted). The tree panel
        // reservation is [0,202] (`exact_size` incl. margins, measured panel.rs:405) minus the
        // 8-px outer margin ⇒ ink 8..202 exclusive = the captured 194-px band x=8..201 (§1);
        // the margin band 0..8 shows `window_fill`, patched to BG_PANEL by `style::apply` ⇒
        // it matches the measured #202020 gutter pixel-for-pixel there.
        assert_eq!(r.tree.min.x, metrics::TREE_PANEL_LEFT_INSET);
        assert_eq!(
            r.tree.max.x,
            metrics::TREE_PANEL_LEFT_INSET + metrics::TREE_PANEL_W_CAPTURE
        );
        // Tree ink rows begin at 79: hairline row 78 is chrome ink itself (§1), so the tree
        // frame carries a 1-px top margin and divider/scrollbar never overpaint the band.
        assert_eq!(r.tree.min.y, r.hairline_y + 1.0);
        // Row-column cursor = ink 8 + inner pad 21 = 29 = the measured glyph-crop left
        // (`work/aida64/out/atlas-analysis.md` part C: tree-glyph x=29..44, y=83..97); the
        // selection ring then starts at 27 (§1) via `paint_tree_row`'s `SELECTION_INSET_X` outset.
        assert_eq!(r.tree.min.x + metrics::TREE_PANEL_INNER_LEFT, 29.0);
        for edge in [
            r.titlebar.min.x,
            r.titlebar.max.y,
            r.hairline_y,
            r.tree.min.x,
            r.tree.max.x,
        ] {
            assert_eq!(
                edge.fract(),
                0.0,
                "band edge {edge} must be device-crisp at PPM 1"
            );
        }

        // Central fills what remains, starting right of the divider column (202 ⇒ ink 203).
        assert_eq!(
            r.central.min.x, 203.0,
            "content begins after divider col 202"
        );
        assert_eq!(r.central.min.y, r.hairline_y + 1.0);
    }

    /// Inside the tree panel the content region is INK − the 21-px inner pad = 173 px; the
    /// measured scrollbar column (17-px trough slot + 4-px gutter = 21 px) is reserved from it,
    /// leaving 152 px of row ink so selection rings can never reach the trough (§2 widths).
    #[test]
    fn scrollbar_slot_is_reserved_from_the_row_column() {
        let slot = metrics::SCROLLBAR_TOTAL_W + metrics::SCROLLBAR_GUTTER_W;
        assert_eq!(slot, 21.0);
        let content_region = metrics::TREE_PANEL_W_CAPTURE - metrics::TREE_PANEL_INNER_LEFT;
        assert_eq!(content_region, 173.0);
        assert_eq!(content_region - slot, 152.0);
    }
}
