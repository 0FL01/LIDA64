//! AIDA64-Extreme (v8.40.8500, English, dark theme) *appearance tokens* for a pixel-perfect Linux
//! port on `egui` (now) and `egui-glow`/`wgpu` (later).
//!
//! ## What this crate is
//! A data + style layer built strictly from measurements of the reference captures, pinned to the
//! same GUI stack the port will use (`egui = "=0.36.2"`, matching
//! `/home/opencode/silver-bullet-work/kit/workspace/Cargo.toml` and its `Cargo.lock`:
//! `egui 0.36.2`, `eframe 0.36.2`, `egui_glow 0.36.2`, `glow 0.17.0`, `wgpu 30.0.1`).
//!
//! * [`palette`] — exact sRGB swatches, incl. the GDI ClearType hue-fringe triplets.
//! * [`metrics`] — pixel geometry (row pitches, indents, widget sizes, grid pitches) at PPM 1.0.
//! * [`style`] — the complete `egui::Style` for the theme, per-region [`egui::Frame`]s and the
//!   per-region style patches egui 0.36 requires (its button/checkbox/separator visuals are
//!   *derived* from `visuals.widgets.<state>`, so single-state values are shared between widgets).
//! * [`paint`] — 1-device-pixel crisp painting for asymmetric frames, dithered hairlines, the tight
//!   tree selection box, graph grids and the scrollbar slot.
//!
//! ## How to use it
//! ```
//! # use aida64_ui_theme::{apply, palette, metrics};
//! let ctx = egui::Context::default();
//! apply(&ctx); // installs the theme as Context's dark style
//! let style = ctx.style_of(egui::Theme::Dark);
//! assert_eq!(style.visuals.panel_fill, palette::BG_PANEL);
//! assert_eq!(style.spacing.indent, metrics::TREE_INDENT);
//! ```
//! Per region, override locally instead of globally (egui 0.36 has no style *stack* push/pop —
//! `Ui::style_mut` / `Ui::set_style` are the mechanisms, per
//! `egui-0.36.2/src/ui.rs:379-396`):
//! ```
//! # use aida64_ui_theme::{aida64_style, patch_checkbox_region};
//! # let mut style = aida64_style();
//! patch_checkbox_region(&mut style); // inside a checkbox-only Ui
//! ```
//!
//! ## Measurement contract (read before changing a constant)
//! * Evidence files: `work/aida64/incoming/ksnip_20261014-003{601,615,720,747}.png` (sha of the
//!   archive tarball `c528f29f…e62c`), tool `work/aida64/incoming/probe.py`, full report with the
//!   exact commands and coordinates: `work/aida64/out/ksnip-20261014-003-ui-spec.md`.
//! * Calibration: `pHYs` 96 DPI ⇒ all metrics are exact at `pixels_per_point() == 1.0`
//!   ([`metrics::CALIBRATION_PPM`]); HiDPI handling is [`paint::snap`].
//! * Every constant must keep its provenance comment (`img1 …`, `img4 …`, `derived:` ,
//!   `unmeasured:`). A value without evidence belongs in `unmeasured:` form, not as a bare literal.
//!
//! ## Known open items (carried from the spec, §6.3/§7 — do not paper over them)
//! 1. Font file + x-height/advance table: only em 12 px / cap 8–9 px is measured, so the theme maps
//!    all text styles to `FontFamily::Proportional` at [`metrics::FONT_EM_PTS`] and installs no TTF.
//!    GDI ClearType LCD fringes ([`palette::FRINGE_ON_PANEL`] and friends) will not appear with
//!    egui's grayscale atlas — accepting that is an explicit, documented divergence.
//! 2. Icon atlas (16 px / 24 px, 1× and 2× variants): the bitmaps are still missing; only the ink
//!    extents and the 20-hue accent palette are measured.
//! 3. img3 skin: diagonal dither gradient (`GRADIENT_TL`→`GRADIENT_BR`, mid `GRADIENT_MID`, ±4) and
//!    the dither-pair hairline alternation period need the raw, non-re-encoded window pixels or the
//!    skin resource extracted from `aida64.exe`/`aida64.dat` (`make pe-resources` on the target).
//! 4. True window geometry, status-bar existence and a 2× capture: `*_CAPTURE` metrics are crop
//!    artifacts, and the two main-window captures are cropped right/bottom.
//! 5. Unmeasured states: pressed/checked-off/unchecked variants, hover fills, disabled *text*
//!    color, log-list row pitch (its list was empty), Copy/Latency column bounds, axis label colors,
//!    value-box x-extents, toolbar control semantics.
//!
//! `unsafe_code` is forbidden crate-wide (mirrors the kit workspace lint) and nothing here panics
//! outside tests.

pub mod compose;
pub mod metrics;
pub mod paint;
pub mod palette;
pub mod style;
pub mod widgets;

pub use style::{
    aida64_style, aida64_style_modifier, apply, button_frame, checkbox_frame, dialog_frame,
    flat_frame, header_strip_frame, img3_table_fill, panel_frame, patch_button_region,
    patch_checkbox_region, patch_disabled_button_region, patch_field_region, selection_frame,
    titlebar_frame, toolbar_frame, value_field_frame,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_crate_exposes_one_theme_and_its_evidence_layers() {
        let s = aida64_style();
        // The three layers must agree with each other (no stale constant left behind).
        assert_eq!(s.visuals.panel_fill, palette::BG_PANEL);
        assert_eq!(s.spacing.indent, metrics::TREE_INDENT);
        assert_eq!(
            s.visuals.selection.stroke.width,
            metrics::SELECTION_STROKE_W
        );
        assert_eq!(s.spacing.icon_width, metrics::CHECKBOX_SIZE);
        assert_eq!(
            s.spacing.scroll.bar_width + 2.0 * s.spacing.scroll.bar_outer_margin,
            metrics::SCROLLBAR_TOTAL_W
        );
        assert_eq!(paint::trace_color(), palette::TRACE_GRAPH);
    }
}
