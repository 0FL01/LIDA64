//! AIDA64-Extreme dark theme as an `egui` 0.36.2 [`Style`], plus per-region [`Frame`]s and
//! per-region style patches.
//!
//! Why the overrides look like this (provenance, all measured):
//! * colors/metrics: `work/aida64/out/ksnip-20261014-003-ui-spec.md` §5, measured on
//!   `work/aida64/incoming/ksnip_20261014-003*.png` at PPM 1.0.
//! * egui-side wiring: read from the pinned crate source at
//!   `/home/opencode/.cargo/registry/src/*/egui-0.36.2`, not from memory or from the report's
//!   §6 guesses (which target older/newer egui APIs). The two derivations that drive most of
//!   this file are `egui-0.36.2/src/widget_style.rs:157-170` (a button's fill is
//!   `weak_bg_fill`, its outline is `bg_stroke`, its size changes come from `expansion`) and
//!   `egui-0.36.2/src/widget_style.rs:212-218` (a separator's stroke is
//!   `visuals.noninteractive().bg_stroke`, its spacing is hard-coded 6.0 and therefore not
//!   style-configurable).
//!
//! What egui *cannot* express with a single `Style`, and therefore must be painted
//! (`crate::paint`) — each of these is a measured fact, not a preference:
//! 1. Asymmetric 1 px window/field frames (img4 dialog: top `#313131`, right `#1A1A1A`,
//!    left/bottom `#202020`; img1 search field: left `#FFFFFF`, others `#7A7A7A`) — egui strokes
//!    are one color all around.
//! 2. Tight tree selection (81 × 18 px) — egui highlights the whole row area, and the
//!    `SELECTED_CLASS` path (`widget_style.rs:150-155`) also re-colors the *text* to the selection
//!    stroke color, while AIDA64 keeps text `#FFFFFF`.
//! 3. Scrollbar trough `#171717` / thumb `#4D4D4D` / arrows `#676767` — the measured trough is
//!    *darker* than the panel, so egui's opacity-over-fg model (which can only lighten `#202020`
//!    with a light hairline color) cannot produce it; and thumb-vs-trough is not a consistent
//!    alpha of the hairline color either (R/G ⇒ 0.526, B ⇒ 0.44).
//! 4. Disabled button outline `#404040` with fill `#333333` — disabled widgets use
//!    `widgets.noninteractive`, which is also the source of every faint hairline (`#828790`), so a
//!    single style state cannot serve both. Use [`patch_disabled_button_region`] where needed.
//! 5. img3 skin (dithered diagonal gradient + borderless teal buttons) — no egui gradient fill
//!    exists; needs a mesh/shader stage (egui-glow) or a per-pixel paint pass.
//!
//! Font: only metrics are known (em 12 px ⇒ cap 8–9 px). The exact TTF is still missing
//! (spec §7 item 1), so the theme installs no font file and maps every text style onto
//! `FontFamily::Proportional` at [`metrics::FONT_EM_PTS`] — the size is measured, the glyphs are
//! not yet. egui's grayscale AA will not reproduce the GDI ClearType hue fringes
//! ([`palette::FRINGE_ON_PANEL`] and friends).

use egui::style::{Selection, StyleModifier, WidgetVisuals};
use egui::widget_style::WidgetState;
use egui::{
    Color32, CornerRadius, FontFamily, FontId, Frame, Margin, Shadow, Stroke, Style, TextStyle,
    vec2,
};

use crate::metrics::*;
use crate::palette::*;

/// One widget-visual state, written with the measured values.
///
/// `outline` feeds `bg_stroke` (button/checkbox frames, faint hairlines), `fg` feeds `fg_stroke`
/// (the tick glyph and icon strokes). Width is always [`BORDER_W`] because every measured border is
/// 1 px; `expansion = 0.0` is required by the measurement that the hot (white-framed) and enabled
/// buttons have the *same* 71 × 19 px fill rect (spec §5, `button` row).
fn set_state(v: &mut WidgetVisuals, fill: Color32, outline: Color32, fg: Color32) {
    v.bg_fill = fill;
    v.weak_bg_fill = fill; // button fill path in egui 0.36 (widget_style.rs:159)
    v.bg_stroke = Stroke::new(BORDER_W, outline);
    v.fg_stroke = Stroke::new(BORDER_W, fg);
    v.corner_radius = CornerRadius::ZERO;
    v.expansion = 0.0;
}

/// Interactive-widget states for buttons (the dominant interactive widget in AIDA64).
///
/// `active` (pressed) and `open` are `unmeasured:` — the captures contain no pressed or open
/// control — so they keep the measured resting fill `#333333` and the measured hot outline
/// `#FFFFFF` / enabled outline `#9B9B9B` respectively, which preserves the measured 71 × 19 px
/// geometry. `hovered` *is* measurable-ish: img4's Start button carries the white outline while its
/// fill stays `#333333` (`stroke.button.hot`).
pub fn patch_button_region(style: &mut Style) {
    let w = &mut style.visuals.widgets;
    set_state(
        &mut w.inactive,
        BG_BUTTON,
        STROKE_BUTTON_ENABLED,
        GLYPH_CHECKBOX_TICK,
    );
    set_state(
        &mut w.hovered,
        BG_BUTTON,
        STROKE_BUTTON_HOT,
        GLYPH_CHECKBOX_TICK,
    );
    set_state(
        &mut w.active,
        BG_BUTTON,
        STROKE_BUTTON_HOT,
        GLYPH_CHECKBOX_TICK,
    );
    set_state(
        &mut w.open,
        BG_BUTTON,
        STROKE_BUTTON_ENABLED,
        GLYPH_CHECKBOX_TICK,
    );
}

/// Checkbox-list states: measured interior is the *panel* background (`#202020`), not the button
/// fill, and the outline is `#D2D2D2`. Push this on a `Ui` that contains only checkboxes
/// (`ui.set_style(Arc<Style>)` or `ui.style_mut()`).
pub fn patch_checkbox_region(style: &mut Style) {
    style.spacing.icon_width = CHECKBOX_SIZE; // CheckboxStyle.checkbox_size (widget_style.rs:179)
    style.spacing.icon_width_inner = CHECKBOX_INTERIOR; // check_size (:180)
    let w = &mut style.visuals.widgets;
    set_state(
        &mut w.inactive,
        BG_PANEL,
        STROKE_CHECKBOX,
        GLYPH_CHECKBOX_TICK,
    );
    set_state(
        &mut w.hovered,
        BG_PANEL,
        STROKE_CHECKBOX,
        GLYPH_CHECKBOX_TICK,
    );
    set_state(
        &mut w.active,
        BG_PANEL,
        STROKE_CHECKBOX,
        GLYPH_CHECKBOX_TICK,
    );
    set_state(&mut w.open, BG_PANEL, STROKE_CHECKBOX, GLYPH_CHECKBOX_TICK);
}

/// Text-field states: measured interior `#202020` with `#7A7A7A` on three edges; the left edge is
/// `#FFFFFF` and must be painted (see [`crate::paint::paint_field_frame`]).
pub fn patch_field_region(style: &mut Style) {
    style.visuals.text_edit_bg_color = Some(BG_FIELD);
    let w = &mut style.visuals.widgets;
    set_state(&mut w.inactive, BG_FIELD, STROKE_FIELD_OTHER, TEXT_PRIMARY);
    set_state(&mut w.hovered, BG_FIELD, STROKE_FIELD_OTHER, TEXT_PRIMARY);
    set_state(&mut w.active, BG_FIELD, STROKE_FIELD_OTHER, TEXT_PRIMARY);
    set_state(&mut w.open, BG_FIELD, STROKE_FIELD_OTHER, TEXT_PRIMARY);
}

/// Disabled-button states (img4 Stop/Preferences): fill `#333333`, outline `#404040`.
/// Push this around disabled buttons only — `widgets.noninteractive` is shared with faint hairlines.
pub fn patch_disabled_button_region(style: &mut Style) {
    set_state(
        &mut style.visuals.widgets.noninteractive,
        BG_BUTTON,
        STROKE_BUTTON_DISABLED,
        STROKE_BUTTON_DISABLED,
    );
}

/// The measured dark theme as a complete `egui::Style`.
#[must_use]
pub fn aida64_style() -> Style {
    let mut style = Style::default();

    {
        let v = &mut style.visuals;
        v.dark_mode = true;
        v.override_text_color = Some(TEXT_PRIMARY); // measured #FFFFFF on every dark fill
        v.panel_fill = BG_PANEL; // toolbar/tree/content/dialog flat #202020
        v.window_fill = BG_PANEL; // img4 dialog flat fill
        v.extreme_bg_color = BG_TITLEBAR; // deepest tone used by titlebar/graphs/value fields
        v.faint_bg_color = BG_HEADER_STRIP; // img4 two-tone header strip
        v.text_edit_bg_color = Some(BG_FIELD); // img1 search field interior
        v.window_stroke = Stroke::new(BORDER_W, STROKE_FRAME_TOP);
        // The measured img4 frame is asymmetric (top #313131 / right #1A1A1A / left+bottom
        // #202020), so window_stroke can only approximate it; paint::paint_dialog_edges is exact.
        v.window_corner_radius = CornerRadius::ZERO; // corner_radius measured 0 everywhere
        v.menu_corner_radius = CornerRadius::ZERO;
        v.window_shadow = Shadow::NONE; // alpha 255 everywhere ⇒ no drop shadows
        v.popup_shadow = Shadow::NONE;
        v.striped = false; // no zebra rows in any capture
        v.button_frame = true; // buttons are filled + outlined, not text-only
        v.collapsing_header_frame = false; // tree rows show no frame fill
        v.indent_has_left_vline = false; // tree band is uniform #202020; no indent guide lines
        v.selection = Selection {
            bg_fill: BG_SELECTION,
            stroke: Stroke::new(SELECTION_STROKE_W, STROKE_SELECTION),
        };
        // weak_text_color / disabled_alpha / text_options / interact_cursor / clip_rect_margin
        // are left at egui defaults: the captures contain no measured disabled glyph text, tooltip
        // or caret pixels, so inventing values here would be guesswork (spec §7 item 5).
        set_state(&mut v.widgets.noninteractive, BG_PANEL, HAIRLINE, HAIRLINE);
        // `open` needs no special treatment beyond what patch_button_region sets; widgets.open is
        // patched there for consistency of the button fill.
    }

    patch_button_region(&mut style);

    {
        let s = &mut style.spacing;
        s.item_spacing = vec2(ICON_LABEL_GAP, 0.0);
        // Vertical item spacing is 0 because every measured row pitch equals its row height
        // (tree 18.0 = TREE_ROW_H, tabs 19.0, checkbox list 20.0): the rhythm is set per region by
        // explicit heights, not accumulated by spacing. Horizontal 2 px is the measured
        // icon→label gap. Button group gaps measured 9–28 px are laid out per button.
        s.icon_spacing = ICON_LABEL_GAP;
        s.indent = TREE_INDENT; // measured 19 px per tree level
        s.icon_width = CHECKBOX_SIZE; // CheckboxStyle.checkbox_size == 13 px (measured)
        s.icon_width_inner = CHECKBOX_INTERIOR; // derived 13 − 2×1
        // button_padding stays at the egui default: it only affects text-derived button sizes, and
        // AIDA64 buttons are fixed 71 × 19 px (metrics::BUTTON_W/H) — set .min_size() explicitly.
        // extra_text_line_spacing stays default: the 15 px tile-label pitch is achieved by
        // explicit layout (LayoutJob / per-line placement), see metrics::TILE_LABEL_LINE_PITCH.
        let sc = &mut s.scroll;
        sc.floating = false; // img2: the 17 px bar is allocated, not floating over content
        sc.bar_width = SCROLLBAR_BODY_W; // 15 px body
        sc.bar_outer_margin = SCROLLBAR_EDGE_W; // reserves the two 1-px #171717 columns
        sc.bar_inner_margin = SCROLLBAR_GUTTER_W; // measured 4-px #202020 gutter (attribution caveat)
        // Trough/thumb/arrow colors are unreachable through ScrollStyle opacities (see the module
        // doc, item 3) and are painted instead: crate::paint::paint_scrollbar_slot.
    }

    {
        let font = FontId {
            size: FONT_EM_PTS,
            family: FontFamily::Proportional,
        };
        for ts in [
            TextStyle::Small,
            TextStyle::Body,
            TextStyle::Button,
            TextStyle::Monospace,
        ] {
            style.text_styles.insert(ts, font.clone());
        }
        // TextStyle::Heading is left default: no capture contains text large enough to pin it
        // (the img4 big numerics measure 10 px cap height, which is a fill/plot label, spec §7).
    }

    style
}

/// Same theme as a [`StyleModifier`] (for `Popup`/`Menu`/`ComboBox` `.style(…)` slots).
#[must_use]
pub fn aida64_style_modifier() -> StyleModifier {
    StyleModifier::from(aida64_style())
}

/// Install the theme as the application's dark style. The light theme is left untouched:
/// the reference set contains no light-theme pixels (spec §5, divergence 5).
pub fn apply(ctx: &egui::Context) {
    ctx.set_style_of(egui::Theme::Dark, aida64_style());
}

// ── Per-region flat frames ───────────────────────────────────────────────────

/// A fill-only, zero-margin, zero-radius frame — the AIDA64 shape language has no corner radius
/// and no padding-based elevation.
pub fn flat_frame(fill: Color32) -> Frame {
    Frame::NONE
        .fill(fill)
        .inner_margin(Margin::ZERO)
        .corner_radius(CornerRadius::ZERO)
}

/// 30 px black caption strip ([`metrics::TITLEBAR_H`]).
pub fn titlebar_frame() -> Frame {
    flat_frame(BG_TITLEBAR)
}

/// 48 px toolbar strip ([`metrics::TOOLBAR_H`]); its 1 px bottom hairline is painted separately
/// ([`crate::paint::paint_hairline_h`]) because it is dithered in the reference.
pub fn toolbar_frame() -> Frame {
    flat_frame(BG_PANEL)
}

/// Tree panel / content panel / central panel background.
pub fn panel_frame() -> Frame {
    flat_frame(BG_PANEL)
}

/// img4 log-list header strip (`#191919`, [`metrics::HEADER_ROW_H`]).
pub fn header_strip_frame() -> Frame {
    flat_frame(BG_HEADER_STRIP)
}

/// img4 value fields / graph panel background (`#000000`). The outline color of the value boxes was
/// not measured (`unmeasured:`), so no stroke is applied.
pub fn value_field_frame() -> Frame {
    Frame::NONE
        .fill(BG_VALUE_FIELD)
        .stroke(Stroke::NONE)
        .inner_margin(Margin::ZERO)
        .corner_radius(CornerRadius::ZERO)
}

/// The tree selection box as a frame: fill `#626262`, 1 px `#3C3C3C` outline, zero padding —
/// matches the measured 81 × 18 px tight box (`SELECTION_BOX_*`). Prefer
/// [`crate::paint::paint_box_1px`] when you need the outline to sit *inside* the rect exactly like
/// the reference (StrokeKind::Inside is not expressible in a `Frame` built from `bg_stroke` alone).
pub fn selection_frame() -> Frame {
    Frame::NONE
        .fill(BG_SELECTION)
        .stroke(Stroke::new(SELECTION_STROKE_W, STROKE_SELECTION))
        .inner_margin(Margin::ZERO)
        .corner_radius(CornerRadius::ZERO)
}

/// Button frame per state: fill always `#333333`; outline carries the state
/// (`#9B9B9B` resting / `#FFFFFF` hot / `#404040` disabled — [`patch_disabled_button_region`] for
/// the disabled case). Zero padding because the button size is fixed by
/// `Button::with_size`/[`metrics::BUTTON_W`] rather than derived from text + padding.
pub fn button_frame(state: WidgetState) -> Frame {
    let outline = match state {
        WidgetState::Inactive => STROKE_BUTTON_ENABLED,
        WidgetState::Hovered | WidgetState::Active => STROKE_BUTTON_HOT,
        WidgetState::Noninteractive => STROKE_BUTTON_DISABLED,
    };
    Frame::NONE
        .fill(BG_BUTTON)
        .stroke(Stroke::new(BORDER_W, outline))
        .inner_margin(Margin::ZERO)
        .corner_radius(CornerRadius::ZERO)
}

/// Checkbox square: 13 × 13 px (`spacing.icon_width`), interior = panel bg, 1 px `#D2D2D2`.
pub fn checkbox_frame(_state: WidgetState) -> Frame {
    Frame::NONE
        .fill(BG_PANEL)
        .stroke(Stroke::new(BORDER_W, STROKE_CHECKBOX))
        .inner_margin(Margin::ZERO)
        .corner_radius(CornerRadius::ZERO)
}

/// img4 dialog contents frame: flat `#202020` with **no** egui stroke; the asymmetric measured
/// edges are painted by [`crate::paint::paint_dialog_edges`].
pub fn dialog_frame() -> Frame {
    flat_frame(BG_PANEL)
}

/// img3 "BenchDLL" table row/cell background is the flat panel tone? No — measured img3 uses the
/// dithered gradient skin; this helper is only for its *text* areas' fills. Table lines are
/// dithered light gray ([`palette::TABLE_LINE_A`]…) and must be painted
/// ([`crate::paint::paint_hairline_h`] with the pair).
pub fn img3_table_fill() -> Frame {
    flat_frame(BG_PANEL)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Color32;

    #[test]
    fn measured_colors_land_in_the_style() {
        let s = aida64_style();
        let v = &s.visuals;
        assert_eq!(v.panel_fill, BG_PANEL);
        assert_eq!(v.window_fill, BG_PANEL);
        assert_eq!(v.extreme_bg_color, BG_TITLEBAR);
        assert_eq!(v.faint_bg_color, BG_HEADER_STRIP);
        assert_eq!(v.text_edit_bg_color, Some(BG_FIELD));
        assert_eq!(v.override_text_color, Some(Color32::WHITE));
        assert_eq!(v.selection.bg_fill, BG_SELECTION);
        assert_eq!(v.selection.stroke.color, STROKE_SELECTION);
        assert_eq!(v.selection.stroke.width, SELECTION_STROKE_W);
        // Separators/faint hairlines come from noninteractive.bg_stroke (widget_style.rs:216).
        assert_eq!(v.widgets.noninteractive.bg_stroke.color, HAIRLINE);
        assert_eq!(v.widgets.noninteractive.bg_fill, BG_PANEL);
        // Button fill is read from weak_bg_fill (widget_style.rs:159) — hence both are set.
        assert_eq!(v.widgets.inactive.weak_bg_fill, BG_BUTTON);
        assert_eq!(v.widgets.inactive.bg_fill, BG_BUTTON);
        assert_eq!(v.widgets.inactive.bg_stroke.color, STROKE_BUTTON_ENABLED);
        assert_eq!(v.widgets.hovered.bg_stroke.color, STROKE_BUTTON_HOT);
        assert_eq!(v.widgets.active.bg_stroke.color, STROKE_BUTTON_HOT);
        assert_eq!(v.widgets.open.bg_stroke.color, STROKE_BUTTON_ENABLED);
        assert_eq!(v.widgets.inactive.fg_stroke.color, GLYPH_CHECKBOX_TICK);
        assert_eq!(v.window_stroke.color, STROKE_FRAME_TOP);
        assert!(v.dark_mode);
    }

    #[test]
    fn everything_is_square_one_pixel_and_flat() {
        let s = aida64_style();
        assert_eq!(s.visuals.window_corner_radius, CornerRadius::ZERO);
        assert_eq!(s.visuals.menu_corner_radius, CornerRadius::ZERO);
        assert_eq!(s.visuals.window_shadow, Shadow::NONE);
        assert_eq!(s.visuals.popup_shadow, Shadow::NONE);
        assert!(!s.visuals.striped);
        assert!(!s.visuals.collapsing_header_frame);
        assert!(!s.visuals.indent_has_left_vline);
        assert!(s.visuals.button_frame);
        for w in [
            &s.visuals.widgets.noninteractive,
            &s.visuals.widgets.inactive,
            &s.visuals.widgets.hovered,
            &s.visuals.widgets.active,
            &s.visuals.widgets.open,
        ] {
            assert_eq!(w.corner_radius, CornerRadius::ZERO);
            assert_eq!(w.expansion, 0.0); // measured button footprint never grows
            assert_eq!(w.bg_stroke.width, BORDER_W);
            assert_eq!(w.fg_stroke.width, BORDER_W);
        }
    }

    #[test]
    fn measured_metrics_reach_spacing() {
        let s = aida64_style();
        assert_eq!(s.spacing.indent, TREE_INDENT);
        assert_eq!(s.spacing.icon_width, CHECKBOX_SIZE);
        assert_eq!(s.spacing.icon_width_inner, CHECKBOX_INTERIOR);
        assert_eq!(s.spacing.icon_spacing, ICON_LABEL_GAP);
        assert_eq!(s.spacing.item_spacing, vec2(ICON_LABEL_GAP, 0.0));
        assert!(!s.spacing.scroll.floating); // measured bar is allocated, not floating
        assert_eq!(s.spacing.scroll.bar_width, SCROLLBAR_BODY_W);
        assert_eq!(s.spacing.scroll.bar_outer_margin, SCROLLBAR_EDGE_W);
        assert_eq!(s.spacing.scroll.bar_inner_margin, SCROLLBAR_GUTTER_W);
        // 17 px total allocation = 15 bar + 2 × 1 outer margin: the measured img2 width.
        assert_eq!(
            s.spacing.scroll.bar_width + 2.0 * s.spacing.scroll.bar_outer_margin,
            SCROLLBAR_TOTAL_W
        );
    }

    #[test]
    fn text_styles_use_the_measured_em() {
        let s = aida64_style();
        for ts in [
            TextStyle::Small,
            TextStyle::Body,
            TextStyle::Button,
            TextStyle::Monospace,
        ] {
            let f = &s.text_styles[&ts];
            assert_eq!(f.size, FONT_EM_PTS);
            assert_eq!(f.family, FontFamily::Proportional);
        }
        // Heading is intentionally left at the egui default (no measured reference).
        assert_ne!(
            s.text_styles.get(&TextStyle::Heading).map(|f| f.size),
            Some(FONT_EM_PTS)
        );
    }

    #[test]
    fn region_patches_move_the_derived_widget_styles_to_measured_values() {
        let mut s = aida64_style();
        patch_checkbox_region(&mut s);
        // CheckboxStyle: checkbox_size = spacing.icon_width, checkbox_frame.fill = state.bg_fill,
        // checkbox_frame.stroke = state.bg_stroke (widget_style.rs:177-189).
        assert_eq!(s.spacing.icon_width, CHECKBOX_SIZE);
        assert_eq!(s.visuals.widgets.inactive.bg_fill, BG_PANEL);
        assert_eq!(s.visuals.widgets.inactive.bg_stroke.color, STROKE_CHECKBOX);
        assert_eq!(
            s.visuals.widgets.inactive.fg_stroke.color,
            GLYPH_CHECKBOX_TICK
        );

        patch_button_region(&mut s);
        assert_eq!(s.visuals.widgets.inactive.bg_fill, BG_BUTTON);
        assert_eq!(
            s.visuals.widgets.inactive.bg_stroke.color,
            STROKE_BUTTON_ENABLED
        );

        patch_field_region(&mut s);
        assert_eq!(s.visuals.text_edit_bg_color, Some(BG_FIELD));
        assert_eq!(
            s.visuals.widgets.inactive.bg_stroke.color,
            STROKE_FIELD_OTHER
        );

        patch_disabled_button_region(&mut s);
        // Disabled-button outline measured #404040 (img4 Stop/Preferences).
        assert_eq!(
            s.visuals.widgets.noninteractive.bg_stroke.color,
            STROKE_BUTTON_DISABLED
        );
        assert_eq!(s.visuals.widgets.noninteractive.bg_fill, BG_BUTTON);
    }

    #[test]
    fn frames_match_measured_fills_and_zero_geometry() {
        assert_eq!(titlebar_frame().fill, BG_TITLEBAR);
        assert_eq!(toolbar_frame().fill, BG_PANEL);
        assert_eq!(panel_frame().fill, BG_PANEL);
        assert_eq!(header_strip_frame().fill, BG_HEADER_STRIP);
        assert_eq!(value_field_frame().fill, BG_VALUE_FIELD);
        assert_eq!(value_field_frame().stroke, Stroke::NONE);
        assert_eq!(dialog_frame().fill, BG_PANEL);
        assert_eq!(selection_frame().fill, BG_SELECTION);
        assert_eq!(
            selection_frame().stroke,
            Stroke::new(SELECTION_STROKE_W, STROKE_SELECTION)
        );
        assert_eq!(checkbox_frame(WidgetState::Inactive).fill, BG_PANEL);
        assert_eq!(checkbox_frame(WidgetState::Inactive).stroke.width, BORDER_W);
        for st in [
            WidgetState::Noninteractive,
            WidgetState::Inactive,
            WidgetState::Hovered,
            WidgetState::Active,
        ] {
            let f = button_frame(st);
            assert_eq!(f.fill, BG_BUTTON); // fill never changes between states (img4)
            assert_eq!(f.stroke.width, BORDER_W);
            assert_eq!(f.corner_radius, CornerRadius::ZERO);
            assert_eq!(f.inner_margin, Margin::ZERO);
        }
        for f in [
            titlebar_frame(),
            toolbar_frame(),
            panel_frame(),
            selection_frame(),
            dialog_frame(),
        ] {
            assert_eq!(f.corner_radius, CornerRadius::ZERO);
            assert_eq!(f.shadow, Shadow::NONE);
        }
    }

    #[test]
    fn theme_is_applicable_as_a_modifier_and_is_deterministic() {
        let _ = aida64_style_modifier(); // compiles against egui 0.36 StyleModifier

        // Field-by-field comparison, excluding `number_formatter`: in egui 0.36 it is
        // `Arc<dyn Fn(f64, RangeInclusive<usize>) -> String>` whose `PartialEq` is `Arc::ptr_eq`
        // (egui-0.36.2/src/style.rs:21-23 and :57-62), so two independently constructed styles can
        // *never* be equal there, while its custom `Debug` prints only the struct name — which is
        // why a whole-`Style` `assert_eq!` fails even when every visual value matches. Its behavior
        // is compared semantically instead.
        fn mismatched(a: &Style, b: &Style) -> Vec<&'static str> {
            let mut out: Vec<&'static str> = Vec::new();
            macro_rules! cmp {
                ($name:ident) => {
                    if a.$name != b.$name {
                        out.push(stringify!($name));
                    }
                };
            }
            cmp!(visuals);
            cmp!(spacing);
            cmp!(interaction);
            cmp!(text_styles);
            cmp!(override_text_style);
            cmp!(override_font_id);
            cmp!(override_text_valign);
            cmp!(drag_value_text_style);
            cmp!(wrap_mode);
            cmp!(animation_time);
            cmp!(debug);
            cmp!(explanation_tooltips);
            cmp!(url_in_tooltip);
            cmp!(always_scroll_the_only_direction);
            cmp!(scroll_animation);
            cmp!(compact_menu_style);
            out
        }
        let sample = 1234.5678_f64;
        let decimals = 2_usize..=3_usize;

        let fresh = aida64_style();
        let m1 = mismatched(&fresh, &aida64_style());
        assert!(
            m1.is_empty(),
            "two freshly built aida64_style() differ in: {m1:?}"
        );
        assert_eq!(
            fresh.number_formatter.format(sample, decimals.clone()),
            aida64_style()
                .number_formatter
                .format(sample, decimals.clone()),
            "number formatting must be deterministic even though its Arc is a fresh allocation"
        );

        let ctx = egui::Context::default();
        apply(&ctx);
        let got = &*ctx.style_of(egui::Theme::Dark);
        let m2 = mismatched(got, &fresh);
        assert!(
            m2.is_empty(),
            "apply() did not install these style fields: {m2:?}"
        );
        assert_eq!(
            got.number_formatter.format(sample, decimals.clone()),
            fresh.number_formatter.format(sample, decimals.clone())
        );
    }
}
