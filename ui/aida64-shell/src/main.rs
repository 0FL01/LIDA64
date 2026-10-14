//! eframe shell that paints the measured AIDA64 chrome from `aida64_ui_theme`.
//!
//! The gate for this crate is COMPILE + CLIPPY + FMT (measured: this host has no display, so the
//! window run is backlog; the pixel-perfect contract lives in `ui/aida64-theme`'s 57-test gate
//! incl. the CPU-raster and GPU composition + texel-perfect-icon lanes). Renderer: wgpu now (the
//! eframe default); the egui_glow lane is the user's next step — flip
//! `eframe::NativeOptions::renderer` to `Glow` and enable the `glow` feature (egui_glow-0.36.2 is
//! unpacked locally, measured).
//!
//! The 16-px tree glyph is loaded at RUNTIME from the atlas evidence dir (`$AIDA64_ATLAS_DIR` or
//! the repo `work/aida64/out/atlas/`, gitignored ⇒ never committed): winner asset
//! `aida_icons10.dll_dib_16x16_32bpp_comp0_pal0_BGRA-alpha_92.png` identified by the golden
//! `atlas-glyph-match` claim (MAD 1.02, `tools/model_vs_capture.py`). A missing asset ⇒ chrome
//! paints without icon ink; nothing is fabricated.

use aida64_ui_theme::compose::paint_aida64_chrome;
use aida64_ui_theme::metrics;
use aida64_ui_theme::style;
use aida64_ui_theme::widgets::{IconPixels, TreeItem};
use eframe::egui;

const WINNER_GLYPH: &str = "aida_icons10.dll_dib_16x16_32bpp_comp0_pal0_BGRA-alpha_92.png";

/// Runtime atlas load (rule 10: binaries stay out of git; provenance: golden claim MAD 1.02).
fn load_winner_glyph() -> Option<IconPixels> {
    let dir = match std::env::var("AIDA64_ATLAS_DIR") {
        Ok(dir) => std::path::PathBuf::from(dir),
        Err(_) => {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../work/aida64/out/atlas")
        }
    };
    let path = dir.join(WINNER_GLYPH);
    match image::open(&path) {
        Ok(opened) => {
            let rgba = opened.into_rgba8();
            Some(IconPixels {
                width: rgba.width() as usize,
                height: rgba.height() as usize,
                rgba: rgba.into_vec(),
            })
        }
        Err(err) => {
            eprintln!(
                "aida64-shell: atlas glyph {} not loadable ({err}) — painting chrome without icon ink",
                path.display()
            );
            None
        }
    }
}

struct AidaShell {
    glyph: Option<IconPixels>,
}

impl AidaShell {
    fn new() -> Self {
        Self {
            glyph: load_winner_glyph(),
        }
    }
}

impl eframe::App for AidaShell {
    // Method name `ui` measured from the eframe-0.36.2 doc-example (`lib.rs:47`:
    // `fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame)`); `update` is the older
    // spelling and the trait lives behind `pub use epi::*` (`lib.rs:167`).
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Measured style + chrome geometry (spec §1 titlebar/toolbar bands, hairline row, tree
        // panel with its 8-px inset + 21-px inner pad, divider column, 17-px scrollbar slot).
        // `TREE_PANEL_W_CAPTURE` 194 is the capture-crop instance — real window/tree widths are
        // measured backlog (true window captures), so they stay parameterized, not baked.
        style::apply(ui.ctx());
        let rows = [
            TreeItem {
                label: "Computer",
                depth: 0,
                expanded: false,
                selected: true,
                icon_rgba: self.glyph.clone(),
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
        let _ = paint_aida64_chrome(
            ui,
            metrics::TREE_PANEL_W_CAPTURE,
            &rows,
            None, // thumb extent unmeasured ⇒ trough slot + arrow zones only
        );
        // Deliberately NOT composed yet (honest omissions, unmeasured): tab strip, ribbon blob,
        // log-list body rows (`egui_extras` Table pitch), status bar — see PLAN backlog.
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([926.0, 668.0]), // img1 crop instance (spec §1 canvas); real window dims = measured backlog
        ..Default::default()
    };
    eframe::run_native(
        "aida64-shell",
        options,
        Box::new(|_cc| Ok(Box::new(AidaShell::new()))),
    )
}
