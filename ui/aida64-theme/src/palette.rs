//! Exact AIDA64-Extreme dark-theme colors, measured from the reference captures.
//!
//! Source of truth: `work/aida64/incoming/ksnip_20261014-003{601,615,720,747}.png`
//! (96 DPI `pHYs` ⇒ calibration at `pixels_per_point() == 1.0`, alpha channel is 255 everywhere,
//! so no shadows / no translucency in the reference). The per-role table and the measurement
//! commands (`work/aida64/incoming/probe.py …`) are documented in
//! `work/aida64/out/ksnip-20261014-003-ui-spec.md` §5 ("Palette-таблица").
//!
//! Every constant here is an *observed* sRGB value; nothing is interpolated unless the doc-comment
//! says so explicitly (`derived:` / `unmeasured:` prefixes).

use egui::Color32;

// ── Backgrounds ──────────────────────────────────────────────────────────────

/// `bg.panel` — toolbar / tree / content / dialog flat background (`#202020`, img1+img2+img4).
pub const BG_PANEL: Color32 = Color32::from_rgb(32, 32, 32);

/// `bg.titlebar` — 30 px caption strip, pure black (`#000000`, img1 y=3..32, img4 y=1..30).
pub const BG_TITLEBAR: Color32 = Color32::from_rgb(0, 0, 0);

/// `bg.header_strip` — log-list header row fill, 24 px (`#191919`, img4). The list body stays
/// `BG_PANEL`, i.e. the two-tone split is header-vs-body, not zebra striping.
pub const BG_HEADER_STRIP: Color32 = Color32::from_rgb(25, 25, 25);

/// `bg.selection` — tight tree-selection box fill (`#626262`, img1).
pub const BG_SELECTION: Color32 = Color32::from_rgb(98, 98, 98);

/// `bg.field` — search-field interior (`#202020`, img1) — same value as [`BG_PANEL`], kept as a
/// separate role because a future light theme will split them.
pub const BG_FIELD: Color32 = Color32::from_rgb(32, 32, 32);

/// `bg.button` — button fill, all states (`#333333`, img4; fill did not change between the
/// white-framed "hot" Start button and the gray-framed enabled ones).
pub const BG_BUTTON: Color32 = Color32::from_rgb(51, 51, 51);

/// `bg.value_field` + `bg.graph` — deepest background used by img4 value boxes and graph panels
/// (`#000000`). Also the titlebar color, hence [`BG_TITLEBAR`] shares it.
pub const BG_VALUE_FIELD: Color32 = Color32::from_rgb(0, 0, 0);

/// `dialog_bg_gradient` endpoints (img3, dithered): TL corner, BR corner, and the
/// anti-diagonal mid tone. Dither amplitude measured ±4.
pub const GRADIENT_TL: Color32 = Color32::from_rgb(62, 71, 115); // #3E4773
pub const GRADIENT_BR: Color32 = Color32::from_rgb(71, 51, 17); // #473311
pub const GRADIENT_MID: Color32 = Color32::from_rgb(64, 64, 64); // #404040 (anti-diagonal)

// ── Strokes / hairlines ──────────────────────────────────────────────────────

/// `stroke.hairline` — the 1 px divider color. Measured as a **dithered pair** that alternates
/// with `#848494`; pixel counts in img1 were `#828790` ×370 vs `#848494` ×938, so the strict
/// 50/50 checker model is *not* established (see spec §7 item 3).
pub const HAIRLINE: Color32 = Color32::from_rgb(130, 135, 144); // #828790
pub const HAIRLINE_ALT: Color32 = Color32::from_rgb(132, 132, 148); // #848494

/// `stroke.selection` — 1 px outline of the tree selection box (`#3C3C3C`, img1).
pub const STROKE_SELECTION: Color32 = Color32::from_rgb(60, 60, 60);

/// `stroke.window.frame.top` — img4 dialog outer frame, top edge (`#313131`).
pub const STROKE_FRAME_TOP: Color32 = Color32::from_rgb(49, 49, 49);

/// `stroke.window.frame.right` — img4 dialog outer frame, right edge (`#1A1A1A`).
/// The measured left/bottom edges equal `BG_PANEL`, i.e. the frame is deliberately asymmetric.
pub const STROKE_FRAME_RIGHT: Color32 = Color32::from_rgb(26, 26, 26);

/// `stroke.button.enabled` — 1 px button outline, resting state (`#9B9B9B`, img4).
pub const STROKE_BUTTON_ENABLED: Color32 = Color32::from_rgb(155, 155, 155);

/// `stroke.button.hot` — outline of the white-framed (default/hot) button (`#FFFFFF`, img4 "Start").
pub const STROKE_BUTTON_HOT: Color32 = Color32::from_rgb(255, 255, 255);

/// `stroke.button.disabled` — outline of disabled buttons (`#404040`, img4 Stop/Preferences).
pub const STROKE_BUTTON_DISABLED: Color32 = Color32::from_rgb(64, 64, 64);

/// `stroke.checkbox` — 1 px checkbox square outline (`#D2D2D2`, img4).
pub const STROKE_CHECKBOX: Color32 = Color32::from_rgb(210, 210, 210);

/// `field.frame.left` — search field left edge is pure white (`#FFFFFF`, img1).
pub const STROKE_FIELD_LEFT: Color32 = Color32::from_rgb(255, 255, 255);

/// `field.frame.other` — search field top/right/bottom edges (`#7A7A7A`, img1).
pub const STROKE_FIELD_OTHER: Color32 = Color32::from_rgb(122, 122, 122);

// ── Scrollbar (img2, only present where content overflows) ───────────────────

/// `bg.scrollbar_trough` — trough plus the two 1 px edge columns (`#171717`, img2).
pub const BG_SCROLLBAR_TROUGH: Color32 = Color32::from_rgb(23, 23, 23);

/// `bg.scrollbar_thumb` — dragged/thumb region (`#4D4D4D`, img2).
pub const BG_SCROLLBAR_THUMB: Color32 = Color32::from_rgb(77, 77, 77);

/// `glyph.scrollbar.arrows` — arrow glyphs inside the 17 px arrow zones (`#676767`, img2).
pub const GLYPH_SCROLLBAR_ARROW: Color32 = Color32::from_rgb(103, 103, 103);

// ── Glyphs / text ────────────────────────────────────────────────────────────

/// `text.primary` — `#FFFFFF` on every dark background (img1+img2+img3+img4).
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(255, 255, 255);

/// `text.footer.gray` — img3 "BenchDLL …" footer line (`#BAB6C2`).
pub const TEXT_FOOTER_GRAY: Color32 = Color32::from_rgb(186, 182, 194);

/// `glyph.checkbox.tick` — tick core (`#9B9B9B`) with an AA ramp up to `#C0C0C0` (img4).
pub const GLYPH_CHECKBOX_TICK: Color32 = Color32::from_rgb(155, 155, 155);
pub const GLYPH_CHECKBOX_TICK_RAMP: Color32 = Color32::from_rgb(192, 192, 192);

/// `grid.graph` — 1 px graph grid lines (`#008000`, img4).
pub const GRID_GRAPH: Color32 = Color32::from_rgb(0, 128, 0);

/// `trace.graph` — plotted trace color (`#FFFF00`, img4).
pub const TRACE_GRAPH: Color32 = Color32::from_rgb(255, 255, 0);

// ── img3 "BenchDLL" skin: dithered teal buttons ──────────────────────────────

/// `bg.button.teal` — endpoints of the dithered teal button fill, no stroke (`#66A696`..`#74B490`,
/// measured with `probe.py runh 3 …`). Mid tones were sampled inside this span only.
pub const TEAL_BUTTON_A: Color32 = Color32::from_rgb(102, 166, 150);
pub const TEAL_BUTTON_B: Color32 = Color32::from_rgb(116, 180, 144);

/// `table_frame` — dithered light-gray table lines of img3 (`#B5B9BD`..`#BAB6C2` span).
pub const TABLE_LINE_A: Color32 = Color32::from_rgb(181, 185, 189);
pub const TABLE_LINE_B: Color32 = Color32::from_rgb(186, 182, 194);

// ── GDI ClearType LCD-subpixel fringe triplets (measured, informational) ─────
//
// These are *outputs* of GDI ClearType on the capture machine, not colors an egui app should
// paint: egui's font atlas is grayscale AA, so reproducing the hue fringe needs a custom
// blending/shader stage (spec §6.3 item 6). They are kept so a pixel-diff harness can whitelist
// them and so a future LCD-subpixel shader has the measured targets.

/// Fringe triplet on `#202020` (img2 ×972/×783/×854, img4 ×779/×654/×732): blue, green, red.
pub const FRINGE_ON_PANEL: [Color32; 3] = [
    Color32::from_rgb(32, 32, 116),   // #202074
    Color32::from_rgb(188, 255, 222), // #BCFFDE
    Color32::from_rgb(153, 76, 32),   // #994C20
];

/// Fringe triplet on the `#626262` selection fill (img1 ×47/×41/×40).
pub const FRINGE_ON_SELECTION: [Color32; 3] = [
    Color32::from_rgb(98, 98, 154),   // #62629A
    Color32::from_rgb(206, 255, 231), // #CEFFE7
    Color32::from_rgb(180, 127, 98),  // #B47F62
];

/// Fringe triplet on the `#333333` button fill (img4).
pub const FRINGE_ON_BUTTON: [Color32; 3] = [
    Color32::from_rgb(51, 51, 127),   // #33337F
    Color32::from_rgb(193, 255, 225), // #C1FFE1
    Color32::from_rgb(160, 90, 51),   // #A05A33
];

/// img3 had **zero** fringe pixels (its text/render path differs) — recorded so a diff harness
/// does not try to explain img3 fringes away.
pub const FRINGE_COUNT_IMG3: usize = 0;

// ── Icon accent palette (tree/toolbar/tile/dialog icons, img1+img2+img4) ─────

/// 20 saturated accent hues measured inside icon blobs. The icon *bitmaps* themselves are still
/// missing (spec §7 item 2); this list is what an extracted atlas must reproduce.
pub const ICON_ACCENTS: [Color32; 20] = [
    Color32::from_rgb(33, 29, 121),   // #211D79
    Color32::from_rgb(33, 113, 193),  // #2171C1
    Color32::from_rgb(28, 76, 156),   // #1C4C9C
    Color32::from_rgb(1, 117, 217),   // #0175D9
    Color32::from_rgb(80, 176, 240),  // #50B0F0
    Color32::from_rgb(112, 200, 255), // #70C8FF
    Color32::from_rgb(3, 171, 3),     // #03AB03
    Color32::from_rgb(34, 98, 34),    // #226222
    Color32::from_rgb(60, 128, 100),  // #3C8064
    Color32::from_rgb(102, 210, 110), // #66D26E
    Color32::from_rgb(254, 166, 30),  // #FEA61E
    Color32::from_rgb(255, 87, 31),   // #FF571F
    Color32::from_rgb(241, 161, 17),  // #F1A111
    Color32::from_rgb(152, 76, 32),   // #984C20
    Color32::from_rgb(189, 113, 37),  // #BD7125
    Color32::from_rgb(223, 151, 79),  // #DF974F
    Color32::from_rgb(186, 254, 226), // #BAFEE2
    Color32::from_rgb(254, 254, 190), // #FEFEBE
    Color32::from_rgb(117, 29, 37),   // #751D25
    Color32::from_rgb(75, 31, 35),    // #4B1F23
];

/// Measured AA ramp example inside a tile icon (`#A87420` → `#FEA61E`, spec §6.7): icon pixels have
/// their own 1 px AA, no dither — reproduced as an atlas property, not a shader.
pub const ICON_AA_RAMP_FROM: Color32 = Color32::from_rgb(168, 116, 32);
pub const ICON_AA_RAMP_TO: Color32 = Color32::from_rgb(254, 166, 30);

#[cfg(test)]
mod tests {
    use super::*;

    /// Independent decode path (hex string → RGB ints) so the constants cannot agree with
    /// themselves by construction.
    fn hex(s: &str) -> Color32 {
        let s = s.trim_start_matches('#');
        let byte = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).expect("valid hex");
        Color32::from_rgb(byte(0), byte(2), byte(4))
    }

    #[test]
    fn palette_matches_report_hexes() {
        // Roles copied verbatim from spec §5 palette table.
        assert_eq!(BG_PANEL, hex("202020"));
        assert_eq!(BG_TITLEBAR, hex("000000"));
        assert_eq!(BG_HEADER_STRIP, hex("191919"));
        assert_eq!(BG_SELECTION, hex("626262"));
        assert_eq!(BG_FIELD, hex("202020"));
        assert_eq!(BG_BUTTON, hex("333333"));
        assert_eq!(BG_VALUE_FIELD, hex("000000"));
        assert_eq!(GRADIENT_TL, hex("3E4773"), "img3 TL corner");
        assert_eq!(GRADIENT_BR, hex("473311"), "img3 BR corner");
        assert_eq!(GRADIENT_MID, hex("404040"), "img3 anti-diagonal");
        assert_eq!(HAIRLINE, hex("828790"));
        assert_eq!(HAIRLINE_ALT, hex("848494"));
        assert_eq!(STROKE_SELECTION, hex("3C3C3C"));
        assert_eq!(STROKE_FRAME_TOP, hex("313131"));
        assert_eq!(STROKE_FRAME_RIGHT, hex("1A1A1A"));
        assert_eq!(STROKE_BUTTON_ENABLED, hex("9B9B9B"));
        assert_eq!(STROKE_BUTTON_HOT, hex("FFFFFF"));
        assert_eq!(STROKE_BUTTON_DISABLED, hex("404040"));
        assert_eq!(STROKE_CHECKBOX, hex("D2D2D2"));
        assert_eq!(STROKE_FIELD_LEFT, hex("FFFFFF"));
        assert_eq!(STROKE_FIELD_OTHER, hex("7A7A7A"));
        assert_eq!(BG_SCROLLBAR_TROUGH, hex("171717"));
        assert_eq!(BG_SCROLLBAR_THUMB, hex("4D4D4D"));
        assert_eq!(GLYPH_SCROLLBAR_ARROW, hex("676767"));
        assert_eq!(TEXT_PRIMARY, hex("FFFFFF"));
        assert_eq!(TEXT_FOOTER_GRAY, hex("BAB6C2"));
        assert_eq!(GLYPH_CHECKBOX_TICK, hex("9B9B9B"));
        assert_eq!(GLYPH_CHECKBOX_TICK_RAMP, hex("C0C0C0"));
        assert_eq!(GRID_GRAPH, hex("008000"));
        assert_eq!(TRACE_GRAPH, hex("FFFF00"));
        assert_eq!(TEAL_BUTTON_A, hex("66A696"));
        assert_eq!(TEAL_BUTTON_B, hex("74B490"));
    }

    #[test]
    fn fringe_and_accent_tables_are_the_measured_hexes() {
        assert_eq!(
            FRINGE_ON_PANEL,
            [hex("202074"), hex("BCFFDE"), hex("994C20")]
        );
        assert_eq!(
            FRINGE_ON_SELECTION,
            [hex("62629A"), hex("CEFFE7"), hex("B47F62")]
        );
        assert_eq!(
            FRINGE_ON_BUTTON,
            [hex("33337F"), hex("C1FFE1"), hex("A05A33")]
        );
        let expected: [Color32; 20] = [
            hex("211D79"),
            hex("2171C1"),
            hex("1C4C9C"),
            hex("0175D9"),
            hex("50B0F0"),
            hex("70C8FF"),
            hex("03AB03"),
            hex("226222"),
            hex("3C8064"),
            hex("66D26E"),
            hex("FEA61E"),
            hex("FF571F"),
            hex("F1A111"),
            hex("984C20"),
            hex("BD7125"),
            hex("DF974F"),
            hex("BAFEE2"),
            hex("FEFEBE"),
            hex("751D25"),
            hex("4B1F23"),
        ];
        assert_eq!(ICON_ACCENTS, expected);
        assert_eq!(ICON_AA_RAMP_FROM, hex("A87420"));
        assert_eq!(ICON_AA_RAMP_TO, hex("FEA61E"));
    }

    #[test]
    fn panel_is_not_the_titlebar_and_selection_is_between_them() {
        // Sanity guard against a copy-paste collapse of the three measured tones.
        assert_ne!(BG_PANEL, BG_TITLEBAR);
        assert_ne!(BG_SELECTION, BG_PANEL);
        // Alpha 255 everywhere was measured on all four captures (no shadows, no translucency),
        // and #202020 is darker than #626262 — both are hard requirements on the swatches.
        assert_eq!(BG_PANEL.to_srgba_unmultiplied(), [32, 32, 32, 255]);
        assert_eq!(BG_SELECTION.to_srgba_unmultiplied(), [98, 98, 98, 255]);
        assert!(BG_PANEL.intensity() < BG_SELECTION.intensity());
    }
}
