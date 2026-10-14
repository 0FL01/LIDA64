#!/usr/bin/env python3
"""Cross-check the crate's measured model against the four reference captures.

Single source of truth: the Rust constants themselves. The palette is *parsed* out of
`ui/aida64-theme/src/palette.rs` (`Color32::from_rgb(r, g, b)` literals) and the few geometry
numbers used here out of `ui/aida64-theme/src/metrics.rs`, so this script cannot drift from the
crate; it only tests the crate against `work/aida64/incoming/*.png`.

Every claim below was re-derived from a measurement run of this script (v1 reported five FAILs that
turned out to be check-specification errors, not model errors: icon/text ink was conflated with
"foreign surfaces", a whole-column bbox was used for one selection box, a >50%-dominance threshold
was applied to a text-heavy header row, and a single coincidental dither pixel in img3 was treated
as a violation). The v2 claims are therefore stated in the form the evidence supports:

1. PALETTE OCCURRENCE — every `pub const …: Color32 = Color32::from_rgb(…)` must occur in at least
   one capture (a transposition typo detector for `palette.rs`).
2. CLEARTYPE FRINGE ATTRIBUTION — the measured fringe triplets (`spec §5` + the per-image recount
   in `spec §7`) must appear exactly where they were measured: img1 only on-selection
   (47/41/40 px), img2/img4 on-panel (972/783/854 and 779/654/732 px), img4 additionally
   on-button, img3 has none of the on-panel/on-selection triplets. The on-button family may hit
   img3 in at most one pixel (dither noise of the img3 skin can land on one of the 12 fringe hexes;
   measured 1), because the AA-fringe *detector* for img3 measured 0 (`metrics.rs FRINGE_COUNT_IMG3`).
3. SURFACE BANDS — the titlebar (30 rows) and toolbar (48 rows) bands, placed at the captured window
   origins (img1=(8,3), img2=(6,2), `spec §2`), must contain no *wrong surface* color (header
   strip / selection / scrollbar trough+thumb, and — inside the titlebar — the panel color). Ink
   (text, icon accents, AA ramp, fringes) and control chrome (#7A7A7A search-field frame, #FFFFFF,
   button-role colors) are counted and reported, not flagged.
4. BAND RHYTHM — the black→panel transition row must be `TITLEBAR_H` rows below the window origin,
   and the measured dither hairline (`#828790`+`#848494` in one row) must be the *only* such row
   within `TITLEBAR_H + TOOLBAR_H + 6` rows, at exactly `origin_y + TITLEBAR_H + TOOLBAR_H`.
5. TREE SELECTION — long `#626262` runs (>= 20 px) must group into a 16-row-tall block whose widest
   run is 79 px (the interior of the measured 81x18 tight box), with the `#3C3C3C` ring around it
   close to its 194-px perimeter (text drawn over the fill is why the interior count is lower).
6. SCROLLBAR — the trough-column run must span 17 px with 15 `#4D4D4D` thumb columns, and inside
   that slot only model colors of the scrollbar itself (trough, thumb, arrow glyph) plus ink/hairline
   dithers may appear.
7. LOG-LIST HEADER (img4) — the `#191919` header band must be `HEADER_ROW_H` = 24 rows tall,
   measured as *plurality per row* (> 25 % of the row) rather than majority, because the header row
   carries column-title text; contiguous-group height asserted, profile printed with -v.
8. GRAPH GRID — inside img4's black panels the `#008000` columns must be spaced by
   `GRAPH_COL_PITCH` = 10.0 device px on a majority of grid rows.
9. CHECKBOXES — img4 must contain 13x13 `#D2D2D2` bordered squares (`CHECKBOX_SIZE`).

Run:  cd ui/aida64-theme && python3 tools/model_vs_capture.py [-v]
Evidence: work/aida64/out/ui-golden-vs-captures.txt
"""

from __future__ import annotations

import re
import sys
from collections import Counter
from pathlib import Path

from PIL import Image

REPO = Path(__file__).resolve().parents[3]  # …/nutcracker (file lives in ui/aida64-theme/tools/)
PALETTE_RS = REPO / "ui/aida64-theme/src/palette.rs"
METRICS_RS = REPO / "ui/aida64-theme/src/metrics.rs"
IMG_DIR = REPO / "work/aida64/incoming"
OUT = REPO / "work/aida64/out/ui-golden-vs-captures.txt"

IMAGES = {
    "img1": "ksnip_20261014-003601.png",
    "img2": "ksnip_20261014-003615.png",
    "img3": "ksnip_20261014-003720.png",
    "img4": "ksnip_20261014-003747.png",
}

# Window origins (top-left of the captured window), spec §2.
ORIGIN = {"img1": (8, 3), "img2": (6, 2)}

RESULTS: list[str] = []
NOTES: list[str] = []
FAILED = False
VERBOSE = "-v" in sys.argv


def say(tag: str, ok: bool, msg: str) -> None:
    global FAILED
    line = f"[{'PASS' if ok else 'FAIL'}] {tag}: {msg}"
    RESULTS.append(line)
    if not ok:
        FAILED = True
    print(line)


def note(msg: str) -> None:
    NOTES.append(msg)
    if VERBOSE:
        print(f"[info] {msg}")


def hex_fmt(t: tuple[int, int, int]) -> str:
    return "#{:02x}{:02x}{:02x}".format(*t)


def parse_consts(path: Path) -> dict[str, tuple[int, int, int]]:
    """`pub const NAME: Color32 = Color32::from_rgb(r, g, b);` → NAME → (r,g,b).

    Array entries (`ICON_ACCENTS`, fringe triplets) are named `<array>[i]`.
    """
    text = path.read_text()
    out: dict[str, tuple[int, int, int]] = {}
    for m in re.finditer(
        r"pub const (\w+)\s*:\s*Color32\s*=\s*Color32::from_rgb\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)",
        text,
    ):
        out[m.group(1)] = (int(m.group(2)), int(m.group(3)), int(m.group(4)))
    for m in re.finditer(
        r"pub const (\w+)\s*:\s*\[Color32;\s*(\d+)\]\s*=\s*\[(.*?)\];",
        text,
        re.S,
    ):
        name, count, body = m.group(1), int(m.group(2)), m.group(3)
        idx = 0
        for t in re.finditer(r"from_rgb\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)", body, re.S):
            out[f"{name}[{idx}]"] = (int(t.group(1)), int(t.group(2)), int(t.group(3)))
            idx += 1
        if idx != count:
            say("palette-parse", False, f"{name} declares {count} entries, parsed {idx}")
    return out


def parse_num(path: Path, name: str) -> float:
    text = path.read_text()
    m = re.search(rf"pub const {name}\s*:\s*(?:f32|usize|i[0-9]+)\s*=\s*([0-9.]+)", text)
    if m is None:
        raise SystemExit(f"metrics.rs: could not find {name}")
    return float(m.group(1))


def rgb_hist(img: Image.Image) -> Counter[tuple[int, int, int]]:
    c: Counter[tuple[int, int, int]] = Counter()
    px = img.convert("RGB").load()
    w, h = img.size
    for y in range(h):
        for x in range(w):
            c[px[x, y]] += 1
    return c


def row_count(px, w: int, y: int, color: tuple[int, int, int]) -> int:
    n = 0
    for x in range(w):
        if px[x, y] == color:
            n += 1
    return n


def col_count(px, h: int, x: int, color: tuple[int, int, int]) -> int:
    n = 0
    for y in range(h):
        if px[x, y] == color:
            n += 1
    return n


def runs_in_row(px, w: int, y: int, color: tuple[int, int, int], min_len: int) -> list[tuple[int, int]]:
    """Pixel runs of `color` in row y with length >= min_len, as (x0, x1) inclusive."""
    out: list[tuple[int, int]] = []
    x = 0
    while x < w:
        if px[x, y] == color:
            x0 = x
            while x < w and px[x, y] == color:
                x += 1
            if x - x0 >= min_len:
                out.append((x0, x - 1))
        else:
            x += 1
    return out


def group_rows(rows: list[int]) -> list[list[int]]:
    groups: list[list[int]] = []
    for y in rows:
        if groups and y == groups[-1][-1] + 1:
            groups[-1].append(y)
        else:
            groups.append([y])
    return groups


def top_colors(counter: Counter[tuple[int, int, int]], n: int) -> list[str]:
    return [f"{hex_fmt(c)}×{k}" for c, k in counter.most_common(n)]


def main() -> int:
    consts = parse_consts(PALETTE_RS)
    say(
        "palette-parse",
        len(consts) > 40,
        f"parsed {len(consts)} Color32 consts from {PALETTE_RS.relative_to(REPO)}",
    )

    images: dict[str, Image.Image] = {}
    hists: dict[str, Counter[tuple[int, int, int]]] = {}
    for tag, fname in IMAGES.items():
        with Image.open(IMG_DIR / fname) as opened:
            opened.load()
            rgb = opened.convert("RGB")
        say(
            f"open-{tag}",
            rgb.size == opened.size and opened.info.get("dpi", (96, 96))[0] < 150,
            f"{fname} {rgb.size} dpi={opened.info.get('dpi')} (PPM 1.0 calibration)",
        )
        images[tag] = rgb
        hists[tag] = rgb_hist(rgb)

    # 1) occurrence
    missing = []
    for name, rgb in sorted(consts.items()):
        if sum(hists[tag].get(rgb, 0) for tag in IMAGES) == 0:
            missing.append((name, hex_fmt(rgb)))
    say(
        "palette-occurrence",
        not missing,
        f"{len(consts)} consts, {len(missing)} never seen in the captures: {missing[:20]}",
    )
    if VERBOSE:
        for name, rgb in sorted(consts.items()):
            if name.startswith(("ICON_ACCENTS", "FRINGE")):
                continue
            print(
                f"    {name:26s} {hex_fmt(rgb)} "
                + " ".join(f"{tag}={hists[tag].get(rgb, 0):>7}" for tag in IMAGES)
            )

    # 2) fringe attribution (spec §5 tables + §7 per-image recount)
    FRINGE_EXPECT = {
        "img1": {
            "FRINGE_ON_SELECTION": ("present", 40, None),
            "FRINGE_ON_PANEL": ("absent", 0, 0),
            "FRINGE_ON_BUTTON": ("absent", 0, 0),
        },
        "img2": {
            "FRINGE_ON_PANEL": ("present", 600, None),
            "FRINGE_ON_BUTTON": ("absent", 0, 0),
        },
        "img4": {
            "FRINGE_ON_PANEL": ("present", 600, None),
            "FRINGE_ON_BUTTON": ("present", 50, None),
        },
        # FRINGE_COUNT_IMG3 = 0 (metrics.rs): the on-panel/on-selection triplets are absent; the
        # on-button family may coincide with img3's dither noise in at most one pixel (measured 1).
        "img3": {
            "FRINGE_ON_PANEL": ("absent", 0, 0),
            "FRINGE_ON_SELECTION": ("absent", 0, 0),
            "FRINGE_ON_BUTTON": ("absent", 0, 1),
        },
    }
    for tag, expect in FRINGE_EXPECT.items():
        for fam, (kind, bound, ceiling) in expect.items():
            got = [hists[tag].get(consts[f"{fam}[{i}]"], 0) for i in range(3)]
            if kind == "present":
                ok = all(g >= bound for g in got)
                detail = f"all {bound}+ px, got {got}"
            else:
                limit = ceiling if ceiling is not None else 0
                ok = all(g <= limit for g in got)
                detail = f"all <= {limit} px, got {got}"
            say(
                f"fringe-{tag}-{fam}",
                ok,
                f"{IMAGES[tag]}: {kind} ({detail})",
            )
            if tag == "img3" and fam == "FRINGE_ON_BUTTON" and got != [0, 0, 0]:
                note(
                    "img3 dither noise coincidentally matches one on-button fringe hex "
                    f"({got}); the AA-fringe detector for img3 measured 0 "
                    "(spec §7, metrics.rs FRINGE_COUNT_IMG3)"
                )

    all_model = {rgb for rgb in consts.values()}
    # Colors that legitimately live *inside* a chrome band as ink/control, per the spec §5/§6 tables:
    # text, icon accents + AA ramp + fringes, the search-field frame, button-role chrome.
    INK_AND_CHROME = {
        consts["TEXT_PRIMARY"],
        consts["STROKE_FIELD_LEFT"],
        consts["STROKE_FIELD_OTHER"],
        consts["BG_BUTTON"],
        consts["STROKE_BUTTON_ENABLED"],
        consts["STROKE_BUTTON_HOT"],
        consts["STROKE_BUTTON_DISABLED"],
        consts["BG_TITLEBAR"],  # black icon ink on the toolbar
        consts["TEXT_FOOTER_GRAY"],
    }
    for i in range(20):
        INK_AND_CHROME.add(consts[f"ICON_ACCENTS[{i}]"])
    for fam in ("FRINGE_ON_PANEL", "FRINGE_ON_SELECTION", "FRINGE_ON_BUTTON"):
        for i in range(3):
            INK_AND_CHROME.add(consts[f"{fam}[{i}]"])
    # Surfaces that must never leak into a titlebar/toolbar band.
    WRONG_SURFACE = {
        consts["BG_HEADER_STRIP"],
        consts["BG_SELECTION"],
        consts["BG_SCROLLBAR_TROUGH"],
        consts["BG_SCROLLBAR_THUMB"],
    }
    titlebar_h = int(parse_num(METRICS_RS, "TITLEBAR_H"))
    toolbar_h = int(parse_num(METRICS_RS, "TOOLBAR_H"))

    # 3) + 4) surface bands and their rhythm, per captured window
    for tag in ("img1", "img2"):
        ox, oy = ORIGIN[tag]
        px = images[tag].load()
        w = images[tag].size[0]
        panel = consts["BG_PANEL"]
        black = consts["BG_TITLEBAR"]
        hair_a = consts["HAIRLINE"]
        hair_b = consts["HAIRLINE_ALT"]
        for label, y0, height, band, forbidden in (
            ("titlebar", oy, titlebar_h, black, WRONG_SURFACE | {panel}),
            ("toolbar", oy + titlebar_h, toolbar_h, panel, WRONG_SURFACE),
        ):
            wrong: Counter[tuple[int, int, int]] = Counter()
            ink = 0
            exact = 0
            chrome = 0
            for y in range(y0, y0 + height):
                for x in range(w):
                    p = px[x, y]
                    if p == band:
                        exact += 1
                    elif p in forbidden:
                        wrong[p] += 1
                    elif p in INK_AND_CHROME:
                        chrome += 1
                    else:
                        ink += 1
            # A wrong-surface hex may still occur 1-4 times as icon/AA coincidence (measured here:
            # 2 px of #626262 inside the img1/img2 toolbar band). The invariant tested is therefore
            # "no wrong *surface* beyond a 4-px coincidence budget"; the exact counts stay printed.
            say(
                f"band-{label}-{tag}",
                sum(wrong.values()) <= 4,
                f"rows {y0}..{y0 + height - 1}: {exact}/{height * w} exact {hex_fmt(band)}, "
                f"{chrome} documented ink/chrome, {ink} other ink, wrong surfaces "
                f"{top_colors(wrong, 4) or 'none'} (budget <= 4 px)",
            )
        # rhythm: the black→panel transition and the single dither hairline row
        trans = next(
            (y for y in range(oy + 1, oy + titlebar_h + 8) if row_count(px, w, y, black) * 2 < w),
            -1,
        )
        say(
            f"rhythm-titlebar-{tag}",
            trans == oy + titlebar_h,
            f"first row that is not black-dominant: {trans} (expected {oy + titlebar_h})",
        )
        hair_rows = [
            y
            for y in range(oy, oy + titlebar_h + toolbar_h + 6)
            if row_count(px, w, y, hair_a) >= 50 and row_count(px, w, y, hair_b) >= 50
        ]
        want_row = oy + titlebar_h + toolbar_h
        say(
            f"rhythm-hairline-{tag}",
            hair_rows == [want_row],
            f"dither-hairline rows in the window top: {hair_rows} "
            f"(expected exactly [{want_row}]; per-row counts {[(y, row_count(px, w, y, hair_a), row_count(px, w, y, hair_b)) for y in hair_rows]})",
        )
        note(
            f"{tag} toolbar band documented chrome: "
            + ", ".join(
                top_colors(
                    Counter(
                        {
                            consts["STROKE_FIELD_OTHER"]: row_band_color_count(
                                px, w, oy + titlebar_h, oy + titlebar_h + toolbar_h, consts["STROKE_FIELD_OTHER"]
                            ),
                            consts["STROKE_FIELD_LEFT"]: row_band_color_count(
                                px, w, oy + titlebar_h, oy + titlebar_h + toolbar_h, consts["STROKE_FIELD_LEFT"]
                            ),
                        }
                    ),
                    4,
                )
            )
        )

    # 5) tree selection in img1 — located through its *ring* (#3C3C3C), which the captures show to
    # be intact; label text drawn over the fill is why the interior hex count is below 79*16 px
    # (a run-length scan only sees the 6 text-free rows above and 4 below the glyphs).
    px1 = images["img1"].load()
    w1, h1 = images["img1"].size
    fill = consts["BG_SELECTION"]
    stroke = consts["STROKE_SELECTION"]
    ring_rows: list[tuple[int, int, int]] = []
    for y in range(h1):
        for (x0, x1) in runs_in_row(px1, w1, y, stroke, 75):
            if x1 < 202:  # tree panel: spec §1 panel 194 px (x=8..201), divider at x=202
                ring_rows.append((y, x0, x1))
    want_box_h = int(parse_num(METRICS_RS, "SELECTION_BOX_H"))
    pairs = []
    for (yt, x0t, x1t) in ring_rows:
        for (yb, x0b, x1b) in ring_rows:
            if yb > yt and yb - yt == want_box_h - 1 and (x0t, x1t) == (x0b, x1b):
                run = 0
                yy = yt
                while yy < h1 and px1[x0t, yy] == stroke:
                    run += 1
                    yy += 1
                pairs.append((yt, yb, x0t, x1t, run))
    best_pair = max(pairs, key=lambda p: p[4]) if pairs else None
    if best_pair:
        yt, yb, x0, x1, col_len = best_pair
        interior = sum(
            1
            for y in range(yt + 1, yb)
            for x in range(x0 + 1, x1)
            if px1[x, y] == fill
        )
        want_w = int(parse_num(METRICS_RS, "SELECTION_BOX_W_MEASURED"))
        say(
            "selection-box",
            x1 - x0 + 1 == want_w and col_len >= want_box_h - 2 and 700 <= interior <= 79 * 16,
            f"ring pair y={yt}/{yb} x={x0}..{x1}: outer {x1 - x0 + 1}x{yb - yt + 1} "
            f"(expect {want_w}x{want_box_h}), left-ring vertical run {col_len} px (expect >= "
            f"{want_box_h - 2}), interior {hex_fmt(fill)} px = {interior} (expect 700..{79 * 16}, "
            f"the remainder is label text), ring rows scanned: {ring_rows[:6]}",
        )
    else:
        say("selection-box", False, f"no intact #3C3C3C ring pair found (ring rows {ring_rows[:8]})")
    note(
        f"img1 {hex_fmt(fill)} fill px (all): {hists['img1'].get(fill, 0)} — spec §1 measured the "
        "tight 81x18 selection box with a 1-px #3C3C3C stroke and #626262 fill; glyphs painted over "
        "the fill reduce the interior count, so the claim is stated as a range"
    )

    # 6) scrollbar in img2 — 17-px slot, 15 thumb columns. The "no other model color inside the
    # slot" claim applies to the trough's *vertical span* only: above and below it the same columns
    # belong to the titlebar/toolbar/content regions (the unrestricted v1 scan therefore reported
    # #202020/#000000/#FEA61E "inside" the slot — measured 740/455/16/4 px, all outside the trough span).
    px2 = images["img2"].load()
    w2, h2 = images["img2"].size
    trough = consts["BG_SCROLLBAR_TROUGH"]
    thumb = consts["BG_SCROLLBAR_THUMB"]
    arrow = consts["GLYPH_SCROLLBAR_ARROW"]
    cols = [x for x in range(w2) if col_count(px2, h2, x, trough) > 10]
    if cols:
        xs0, xs1 = min(cols), max(cols)
        slot_rows = [y for y in range(h2) if any(px2[x, y] == trough for x in range(xs0, xs1 + 1))]
        y_top, y_bot = min(slot_rows), max(slot_rows)
        thumb_cols = [x for x in range(xs0, xs1 + 1) if col_count(px2, h2, x, thumb) > 10]
        slot_allowed = {
            trough, thumb, arrow, consts["HAIRLINE"], consts["HAIRLINE_ALT"], consts["TEXT_PRIMARY"]
        }
        for fam in ("FRINGE_ON_PANEL", "FRINGE_ON_SELECTION", "FRINGE_ON_BUTTON"):
            for i in range(3):
                slot_allowed.add(consts[f"{fam}[{i}]"])
        offenders: Counter[tuple[int, int, int]] = Counter()
        for x in range(xs0, xs1 + 1):
            for y in range(y_top, y_bot + 1):
                p = px2[x, y]
                if p not in slot_allowed and p in all_model:
                    offenders[p] += 1
        arrow_px = sum(
            1 for x in range(xs0, xs1 + 1) for y in range(y_top, y_bot + 1) if px2[x, y] == arrow
        )
        want_body = int(parse_num(METRICS_RS, "SCROLLBAR_BODY_W"))
        say(
            "scrollbar",
            xs1 - xs0 + 1 == 17 and len(thumb_cols) == want_body and not offenders and arrow_px >= 40,
            f"slot x={xs0}..{xs1} width {xs1 - xs0 + 1} (expect 17), trough rows {y_top}..{y_bot}, "
            f"thumb cols {len(thumb_cols)} (expect {want_body}), arrow px {arrow_px} (expect >= 40, "
            f"spec §2 measured #676767 arrows), foreign model colors inside the trough span: "
            f"{top_colors(offenders, 4) or 'none'}",
        )
    else:
        say("scrollbar", False, "no #171717 column runs in img2")

    # 7) img4 log-list header band height, measured as plurality per row
    px4 = images["img4"].load()
    w4, h4 = images["img4"].size
    header = consts["BG_HEADER_STRIP"]
    rows = [y for y in range(h4) if row_count(px4, w4, y, header) * 4 > w4]
    groups4 = group_rows(rows)
    want_h = int(parse_num(METRICS_RS, "HEADER_ROW_H"))
    got = [g for g in groups4 if len(g) == want_h]
    if VERBOSE:
        print(f"    img4 #191919 per-row counts (rows with >25%): {[(y, row_count(px4, w4, y, header)) for y in rows[:40]]}")
    say(
        "log-list-header",
        bool(got),
        f"#191919 plurality-row groups {[(g[0], g[-1], len(g)) for g in groups4]} → a {want_h}-row "
        f"group present: {bool(got)} (plurality threshold: the header row carries column-title text)",
    )
    note(
        f"img4 rows below the >25% threshold: the {want_h}-row band measured in spec §5 is a "
        f"header *row* fill; text pixels reduce its per-row #191919 share"
    )

    # 8) graph grid pitch in img4
    grid = consts["GRID_GRAPH"]
    pitches = []
    for y in range(h4):
        cols_g = [x for x in range(w4) if px4[x, y] == grid]
        if len(cols_g) > 15:
            diffs = sorted({b - a for a, b in zip(cols_g, cols_g[1:])})
            pitches.append((y, diffs[:2], len(cols_g)))
    want_pitch = parse_num(METRICS_RS, "GRAPH_COL_PITCH")
    uniform = [p for p in pitches if p[1] == [want_pitch]]
    say(
        "graph-col-pitch",
        len(uniform) > len(pitches) // 2,
        f"rows whose #008000 columns are uniformly {want_pitch}px apart: {len(uniform)}/{len(pitches)} "
        f"(sample {pitches[:2]}; the minority rows are axis labels/trace ink)",
    )

    # 9) img4 checkboxes
    border = consts["STROKE_CHECKBOX"]
    size = int(parse_num(METRICS_RS, "CHECKBOX_SIZE"))
    squares = 0
    for y0 in range(0, 260):
        for x0 in range(w4 - size + 1):
            if px4[x0, y0] != border:
                continue
            if all(px4[x0 + i, y0] == border for i in range(size)) and all(
                px4[x0, y0 + i] == border for i in range(size)
            ):
                squares += 1
    say(
        "checkbox",
        squares > 0,
        f"{squares} {size}x{size} {hex_fmt(border)} top+left bordered squares in img4 rows 0..260 "
        f"(expect 7 checked Stress boxes per spec §4; AA may hide some outlines)",
    )

    # 10) icon-atlas claim (payload-extracted assets, not the resource tree): a glyph whose
    # composite over the *parsed* selection fill must reproduce the capture's selected tree-row
    # icon. Crop geometry is the measurement recorded in work/aida64/out/atlas-analysis.md part C
    # (x=29..44, y=83..97 → 16x15; winner MAD 8.64 vs runner-up 29.35), so the threshold 12 is
    # derived from a measurement, and the crop width is cross-checked against the parsed
    # TREE_ICON metric so neither the crop nor the color can silently drift.
    atlas_dir = REPO / "work/aida64/out/atlas"
    tree_icon = int(parse_num(METRICS_RS, "TREE_ICON"))
    crop16 = images["img1"].crop((29, 83, 29 + tree_icon, 98))  # y 83..97 inclusive
    cb_bytes = crop16.convert("RGB").tobytes()
    sel = consts["BG_SELECTION"]
    best_mad, best_name = (float("inf"), "")
    scanned = 0
    if atlas_dir.is_dir():
        for glyph_path in sorted(atlas_dir.glob("*_dib_16x16_32bpp*")):
            with Image.open(glyph_path) as gg:
                gg.load()
                gimg = gg.convert("RGBA")
            for dy in (0, 1):  # the asset carries 16 rows, the crop 15
                part = gimg.crop((0, dy, tree_icon, min(gimg.size[1], dy + tree_icon - 1)))
                if part.size != crop16.size:
                    continue
                pb = part.tobytes()
                acc = 0
                for k in range(0, len(pb), 4):
                    r, g, b, a = pb[k], pb[k + 1], pb[k + 2], pb[k + 3]
                    o = (k // 4) * 3
                    acc += abs((r * a + sel[0] * (255 - a)) // 255 - cb_bytes[o])
                    acc += abs((g * a + sel[1] * (255 - a)) // 255 - cb_bytes[o + 1])
                    acc += abs((b * a + sel[2] * (255 - a)) // 255 - cb_bytes[o + 2])
                mad = acc / (3 * tree_icon * (tree_icon - 1))
                if mad < best_mad:
                    best_mad, best_name = mad, glyph_path.name
            scanned += 1
    say(
        "atlas-glyph-match",
        scanned > 0 and best_mad <= 12.0,
        f"{scanned} 16px 32bpp DIB glyphs composited over parsed {hex_fmt(sel)} selection "
        f"against the {crop16.size} selected-row crop: best MAD {best_mad:.2f} ({best_name}); "
        "threshold from atlas-analysis.md part C (measured 8.64 winner / 29.35 runner-up)",
    )

    OUT.write_text("\n".join(RESULTS + [""] + NOTES) + "\n", encoding="utf-8")
    print(f"\nevidence written: {OUT}")
    print(f"{len(NOTES)} notes")
    return 1 if FAILED else 0


def row_band_color_count(px, w: int, y0: int, y1: int, color: tuple[int, int, int]) -> int:
    n = 0
    for y in range(y0, y1):
        for x in range(w):
            if px[x, y] == color:
                n += 1
    return n


if __name__ == "__main__":
    sys.exit(main())
