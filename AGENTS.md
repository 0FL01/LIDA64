# AGENTS.md — nutcracker RE toolkit (agent-facing contract)

Repo: минимальная Docker-only среда реверс-инжиниринга (первый target — AIDA64 Extreme portable).
На хосте нужны **только Docker + Compose v2 + GNU make**. Всё остальное (Node, JDK, Ghidra, `rea`,
`upx`, `jq`, `rg`, `file`, `binutils`, `procps`) — в образе `nutcracker-rea:latest` (2.73 GB).

`rea/` — vendored REA checkout v6.4.0 (reference-код). **Его не редактировать**; для правил по REA-коду
и его тестам читается `rea/AGENTS.md`. Этот файл описывает только toolkit-слой (`docker/`, `Makefile`,
`recipes/`, `PLAN.md`, `work/`).

## Layout

| Путь | Роль |
| ---- | ---- |
| `docker/Dockerfile` | образ + **единственный источник pin'ов** (`ARG`-блок вверху) + build-тайм гейт |
| `docker/compose.yaml` | один сервис `rea` (`platform: linux/amd64`, `mem_limit ${REA_MEM:-4GiB}`, bind `../work:/work`, volume `rea_home:/root/.rea`, портов нет) |
| `docker/selftest.sh` | selftest в двух режимах: `core` (executes при сборке) и `full` (PE/UPX + boundaries + paging) |
| `docker/pin.sh` | `make pin` / `make pin-check` — сверка/обновление sha-пинов с upstream |
| `Makefile` | **канонический entry point** всех рецептов (`make help` полный список) |
| `recipes/00-build-selftest.md` | сборка, readiness-проверки, bootstrap Ghidra-digest, rootless-ownership |
| `recipes/01-aida64-static.md` | AIDA64: UPX-разупаковка → Ghidra overview → search/trace/function → paging → tuning knobs |
| `recipes/02-snapshots-evidence.md` | cost-модель snapshot'ов, Evidence record-vs-bundle boundary, token-economy флаги |
| `PLAN.md` | contract: done-criteria с измеренными доказательствами, out-of-scope, evidence-лог |
| `ui/aida64-theme/` | Rust-краст пиксель-темы AIDA64 (egui `=0.36.2`): `src/{palette,metrics,style,paint,widgets,compose}.rs` + `tests/{pixel_grid,gpu_snapshot}.rs` (CPU-растровый+GPU device-px геймы контракта); все константы = измеренные значения из `work/aida64/out/ksnip-20261014-003-ui-spec.md` |
| `ui/aida64-shell/` | eframe-краст AIDA64-окна (wgpu по default-features; glow-lane позже): `src/main.rs` = runtime-Atlas-load winner-глиф → `paint_aida64_chrome`; compile-gate без дисплея; пиксель-контракт живёт в `ui/aida64-theme` |
| `work/` | gitignored: targets, Evidence JSON, snapshots, logs (`work/<target>/out|analysis|logs`) |

## Канонические команды

```bash
docker compose -f docker/compose.yaml build   # или `make build`
make selftest            # full (ELF + Windows-PE/UPX + Evidence boundary + paging)
make selftest-core       # только build-гейт (то же, что выполняет сборка)
make doctor | providers | capabilities          # assertions/jq выполняются ВНУТРИ контейнера
make unpack SRC=aida64.exe                     # upx -d → <name>.unpacked.exe (source bytes untouched)
TIMEOUT=1800000 MEM=6g MAXMEM=2G SNAPSHOT=aida64-unpacked.json \
  TARGET=aida64 BIN=AIDA64/aida64.unpacked.exe make analyze
TARGET=aida64 BIN=AIDA64/aida64.unpacked.exe SNAPSHOT=aida64-unpacked.json NAME=entry make function
TARGET=aida64 BIN=AIDA64/aida64.unpacked.exe SNAPSHOT=aida64-unpacked.json QUERY=licens make search
TARGET=aida64 make analysis-view FILE=function-aida64.unpacked.exe-entry.json FACET=pseudocode
make bytes ADDR=0x401000 LENGTH=64 | make offset ADDR=0x401000     # engine-free, без --snapshot
make pe-resources | managed-artifact | load-image                  # built-ins
make chown                                                          # после writes контейнером
make pin-check                                                      # reproduce-проверка digests
cd ui/aida64-theme && CARGO_HOME=/home/opencode/.cargo cargo build && cargo test \
  && cargo clippy --all-targets && cargo fmt --check                # оффлайн-гейм UI-темы (57 tests: 31 unit incl. compose+icon + 12 CPU-raster + 12 GPU + 2 doctest)
cd ui/aida64-shell && CARGO_HOME=/home/opencode/.cargo cargo build && cargo clippy --all-targets && cargo fmt --check   # eframe shell (no display ⇒ compile-gate; window run = backlog)
cd ui/aida64-theme && cargo test --test gpu_snapshot -- --test-threads=1   # GPU-lane в serialization
cd ui/aida64-theme && python3 tools/model_vs_capture.py            # golden-сверка модели с captures
```

## Жёсткие правила (нарушение = fail-closed; всё измерено)

1. **Пути.** Targets/Evidence lives в bind-mounted `./work`; в командах REA только контейнерные
   absolute paths `/work/<target>/…` (они пишутся в Evidence → bundle переносим между машинами).
   `SRC`/`unpack` — relative к `work/<TARGET>/`; `BIN` — relative к `/work/<TARGET>/` (например
   `TARGET=aida64 BIN=AIDA64/aida64.unpacked.exe` → `/work/aida64/AIDA64/aida64.unpacked.exe`).
   Выводы `make` пишутся host-side: `work/<TARGET>/out/<cmd>-<name>.json`, snapshots — `work/<TARGET>/analysis/`.
   `$(UNPACKED)` = `$(subst .exe,.unpacked.exe,$(SRC))` → packed-inputs должны называться `*.exe`.
2. **Env knobs.** `REA_*`/`GHIDRA_*` передаются ТОЛЬКО непустыми: zod-схема `rea/src/config/environment.ts`
   reject'ит пустые значения (`make providers LANGUAGE_ID=` → exit 0 потому что флаг пропущен;
   `docker compose run -e REA_GHIDRA_LANGUAGE_ID= rea …` → `rea exit 1`).
   **`-e` must precede the service name** в `compose run`. Symptom misplaced flags:
   `{"code":"UNKNOWN","message":"Unknown flag: -e"}` (это было real bug: дублирование `$(GHIDRA_ENV)`
   в 6 targets → fixed `--provider ghidra` lines).
   `LANGUAGE_ID` требует парного `COMPILER_SPEC_ID` (`make check-args`);
   knobs: `TIMEOUT`=`REA_GHIDRA_STARTUP_TIMEOUT_MS` (default 330 000), `MAXMEM`=`GHIDRA_HEADLESS_MAXMEM`
   (baked 2G; единственный supported memory knob — `_JAVA_OPTIONS`/`JAVA_TOOL_OPTIONS` REA clears),
   `DISABLED_ANALYZERS` (exact Ghidra analyzer names, PE-ресурсы: `WindowsResourceReference`),
   `SEED_FILE` = absolute path to TSV `0xADDR<TAB>function|code|label[<TAB>name]` (`#` comments).
3. **Образ — self-proving, но скрипт живёт в образе.** `make selftest` выполняет **image copy**
   `/usr/local/selftest.sh`; после правок `docker/selftest.sh` must `make build` first
   (это дважды стоило failed runs). Изменения `Makefile`/`recipes` не требуют rebuild; изменения
   `docker/Dockerfile` до REA-слоя инвалидируют тяжёлые слои (ripgrep-слой on purpose placed after
   pinned tool layers, чтобы пересборка была дешёвой).
4. **Snapshot/cache semantics (measured).** `--snapshot` serves только identical queries
   (target digest + operation + parameters + provider + settings). Warm-eligible:
   `analyze`, `search`, `function`, `trace` (~0.2–2.2 s). Всегда provider-cold, даже с snapshot:
   `decompile`, `instructions`, `xrefs`, `inspect-native-load-image`. `read-bytes`/`address-to-file-offset`
   reject'ят `--snapshot` (`Unknown flag`). Имя snapshot-файла не identity-критично (cp-копия тоже warm-hit).
   После `upx -d` (digest изменился) — fresh snapshot; metadata-writes invalidate active cache;
   close/reopen before saving another snapshot. Source: `rea/src/application/binary/AnalysisSnapshotCache.ts`,
   `rea/src/ghidra/GhidraProviderCapabilities.ts`, `rea/docs/cli.md:177,185-189`.
5. **Evidence boundary.** CLI `--json` = ОДНА Evidence-**record**, не bundle: `evidence-export`/`compare`
   on records fail closed (`evidence_integrity_mismatch` — «Expected an Evidence bundle with records and
   manifest arrays…»); bundles produces MCP-tool `export_evidence_bundle` или saved bundle file.
   Paging: `rea inspect-analysis-view` требует **positional** `inputJson` (piping → CLI-layer
   `VALIDATION_ERROR` + `fieldErrors`); envelope `{"source":{"kind":"inline","evidence":…},"view":{…}}`;
   `view:{kind:"native",facet:…}` accepted только для parent-ops `analyze_function`,
   `analyze_javascript_application`, `inspect_binary_layout` (`summary` на `binary_overview`-rec →
   `unsupported_target`); facets: pseudocode, procedure, assembly, basic_blocks, comments, callers, callees,
   references, unresolved-calls, referenced-strings, referenced-names, native-api, high-pcode;
   `item.unit = "utf16-code-units"`. Domain-ошибки = lowercase codes + `category`/`remediation`;
   CLI-arg-ошибки = uppercase + `fieldErrors` — не путать.
6. **Ownership (rootless Docker, measured here).** Container writes land as daemon-owner host uid;
   `make chown` = `chown -R 0:0 /work` inside → host sees your uid. Chown к хост-uid *внутри* контейнера
   попадает на ДРУГОЙ хост-uid (offset mapping) и ломает host-side writes. Root-run daemon → host:
   `sudo chown -R <you> work`. Host-side `mkdir` в `work/` допустим только когда `work/` host-owned
   (`make workdirs` использует host `mkdir`).
7. **Timing/observing.** REA пишет pino-JSON в **stderr** (`"durationMs":`, `"status":"ok"`) — stdout чист
   для `--json`. Capture: `… >/dev/null 2>work/<t>/logs/x.err` затем **`rg -o '"durationMs":[0-9.]+'`**
   (ripgrep: host 14.1.1 и **в образе** с этого commit'а; `grep` — banned for scanning, медленный на big files).
   `docker compose run` noise ("Container … Creating/Created") тоже в stderr → `rg -v Creating|Created`.
   Долгие runs → background job + poll (`cat work/<t>/logs/aida_chain.log`), потому что tool timeout kill'ит
   foreground (killed container terminates cleanly, snapshot может остаться valid). Shell = zsh → multi-op
   batches через `bash -c '…'`; globbing паттерны могут not match → guard glob expressions.
8. **Pins.** Единственный источник — `ARG`-блок в `docker/Dockerfile` (Node/JDK/Ghidra digests,
   `REA_PKG_VERSION`). Ghidra не publish'ит checksum → digest bootstrap'ится stream-хешем official asset
   (`GHIDRA_SHA256=` empty → build печатает digest и fail'ит с инструкцией; SourceForge mirror — comment-fallback).
   Bump procedure: `make pin` → `make build` → `make selftest`. Никогда не hard-code'ить digest вне Dockerfile.
9. **Capabilities semantics.** Scoped doctor — the gate: `rea doctor --provider ghidra --json` →
   `healthy:true` + 9/9 `scope_checks[].ok:true` (keys: `name/ok/classification/detail`, НЕ `id/healthy`).
   Full audit healthy=false — нормально: 4 informational checks (deliberately absent capabilities).
   `providers/capabilities --json` shape: `analysis_provider_candidates[]` (hopper `executable_missing`,
   ida `not_configured` → `REA_IDA_MCP_CONFIG`), `capabilities` count 18; npm-пакет 6.4.0 lacks
   `inspect-go-binary` (main-only) — не promising его в recipes.
10. **Commit hygiene.** Never commit binaries/targets/Evidence (`work/*` gitignored, `!work/.gitkeep`);
    `rea/` — submodule-like vendored checkout, status must stay clean (`git -C rea status --porcelain`
    empty); repo-root `.git` was created by `git init -q` during implementation (no commits yet).
11. **UI-тема (`ui/aida64-theme`).** Пиксель-тема строится **только** из измеренных значений
    `work/aida64/out/ksnip-20261014-003-ui-spec.md` (calibration `PPM = 1.0`, pHYs 96 DPI): каждый
    const в `palette.rs`/`metrics.rs` обязан иметь в комментарии путь к артефакту-доказательству;
    new values without a measurement are forbidden (add the measurement first). Crisp 1-px geometry
    is painted as `fill_rect`s snapped to the device grid — `Stroke`/`Shadow`/rounded corners are
    feathered or unmeasured and must not enter the AIDA64 paint layer; the raster harness
    (`tests/pixel_grid.rs`, 12 tests) enforces this by asking egui what it really queued.
    egui pinned `=0.36.2` (`Cargo.lock` = pin-источник, analog Dockerfile ARG); cargo runs with
    `CARGO_HOME=/home/opencode/.cargo` (offline, registry sources of `egui/eframe/egui_glow/…`
    pre-unpacked there); `rust-version 1.95`, `unsafe_code = "forbid"`. egui 0.36.2 has **no**
    `TopBottomPanel` (use `egui::Panel::top|bottom|left|right`, `.show` not `.show_inside`), no
    Tree/TabBar/Table (Table = `egui_extras`), and device scale = `zoom_factor ×
    ViewportInfo.native_pixels_per_point` — measured 2.0+`set_pixels_per_point(2.0)` = 4.0 drift.
    `TexturesDelta::drop` panics on unapplied deltas → harnesses must `clear()` it.
    **GPU-lane** (`tests/gpu_snapshot.rs`, 12 tests: 9 crisp-prim + composition dump/bands/
    texel-perfect icon): dev-deps `egui_kittest = "=0.36.2"
    features [snapshot,wgpu]` + `egui-wgpu = "=0.36.2"` (HYPHEN crate name — `egui_wgpu` fails
    resolution, measured) + `image = "0.25", default-features = false, features = ["png"]`
    (image's default `default-formats` pulls `exr`, which is absent from the local index cache —
    measured `ui-test-17.err`; the `png` feature is for runtime atlas decode in the icon test;
    network egress works, so cargo runs without `--offline`). Raster runs through
    headless **lavapipe** (`/usr/share/vulkan/icd.d/lvp_icd.json`) and is byte-deterministic
    (`RendererOptions::PREDICTABLE`). Measured harness facts: egui reserves an **8-point content
    inset** (8 device px @PPM 1, 16 @PPM 2); a `CentralPanel` painter is **clipped** to its content
    rect; readback clear color is transparent `#000000/00`; a filled rect that is one device px in
    *both* dimensions tessellates to a zero-width core and renders **0 px** (`ui-test-25.out`),
    while 1-px-thick *long* rects raster exact with zero AA — so the per-device-pixel dither pair is
    a CPU-lane contract only. `Ui::painter()` returns `&Painter` (`egui-0.36.2/src/ui.rs:458`) —
    write `p`, not `&p`. Concurrent lavapipe devices SIGSEGV the test process (`ui-test-29.err`), so
    `render_scene` holds a module `Mutex`; `--test-threads=1` is the fallback. `cargo clippy --fix`
    needs `--allow-dirty` (repo root has zero commits). `ctx.tessellate` panics "No fonts loaded" on
    a pristine context → prime with an empty `run_ui` pass first.
    **Textured lane (measured, ui-test-41..64):** kittest `Harness::render()` never applies
    `TexturesDelta`, egui 0.36 drains alloc-deltas DURING `run_ui` (`load_texture` = manager-alloc
    only, `context.rs:2390-2410`; `take_delta` is the render-side handoff) ⇒ readback sees EMPTY
    manager + last-pass output deltas (`ui-test-63.out` `delta set keys []`), and
    `TextureManager::Default` pre-allocates 1×1 WHITE at `Managed(0)` (`textures.rs:17-18`) that
    solid fills sample via `WHITE_UV` ⇒ without manual uploads NOTHING textured rasterizes there
    (fills included). `render_scene_with_textures` fixes it eframe-style: drains
    `ctx.tex_manager().write().take_delta()` (E0716 fix: bind the Arc first), then uploads
    `ImageDelta::full` for WHITE@default + every glyph the crate stashes (`icon_texture_uploads()`;
    `ICON_TEX_SEQ` makes names unique per call ⇒ every settle pass allocates+stashes;
    `Harness::builder().build_ui` runs the app ≥5× — measured seq 1..5 ⇒ "last-pass" wiring gates
    unreliable). The lane delivers solids + uploaded textures but NOT egui's managed font atlas ⇒
    no text-ink asserts there; eframe unaffected. API facts: `TexturesDelta` field is `set`;
    `ClippedPrimitive` field is singular `primitive`; `Primitive::Mesh(Mesh)` → `Mesh.texture_id`;
    `TextureId` Debug-not-Display; `image` with default-features off decodes to `into_vec()` not
    `into_bytes()` (`ui-test-44`). `icon_slot` = 16×16 vertically centered (GPU-measured
    [37,88)-[53,104); horizontal 16 @ cursor = measured crop x=29..44; exact capture vertical
    correspondence pending window/2× captures ⇒ layout choice; NO CPU test references icon_slot).
    **eframe shell (`ui/aida64-shell`, compile-verified first-try `ui-shell-build-1` exit 0):**
    eframe 0.36.2 `App` trait method is `ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame)`
    — NOT `update` (doc-example `lib.rs:47`; trait via `pub use epi::*` :167) — compiler-verified;
    `eframe::egui` re-export (:156) ⇒ the crate needs no separate egui dep; `run_native` exists with
    default features because default `wgpu` implies `wgpu_no_default_features` (cfg :286).
    `ViewportBuilder::default().with_inner_size([926.0, 668.0])` compiles (img1-crop instance;
    real window dims = measured backlog). Atlas glyphs load at RUNTIME (`AIDA64_ATLAS_DIR` env
    override else manifest `../../work/aida64/out/atlas`, `image::open` → `into_rgba8` →
    `into_vec`); missing glyph ⇒ eprintln + chrome-without-icon degradation (no fabricated ink).
    egui_extras not added until log-list pitch measured; gate = build+clippy+fmt only (no display
    here ⇒ window run backlog); glow lane = `NativeOptions.renderer` Glow + enable `glow`
    (`egui_glow-0.36.2` unpacked locally).
    Golden evidence: `tools/model_vs_capture.py` (PIL-only, parses consts from `src/palette.rs`/
    `src/metrics.rs` so it cannot drift) → `work/aida64/out/ui-golden-vs-captures.txt` (30 PASS:
    palette occurrence 65/65, per-image fringe attribution, titlebar/toolbar band counts, rhythm rows
    33/32 and the only dither row 81/80, selection ring exactly 81×18, scrollbar slot 17-px clean
    inside the trough span, img4 header 24 rows, 10.0-px graph pitch, 13×13 checkboxes).
    **FinalWire resource dialect + unlocated skin colors (measured):** REA `inspect-pe-resources`
    reports `resources 0 / icon_groups 0 / directories null` for every FinalWire UI/icon module
    (`aida_uires.dll`, `aida_uireshd.dll`, `aida_icons10.dll`, `aida_icons2k.dll`,
    `aida64.unpacked.exe`); the packed `aida64.exe` errors `invalid_request` «Resource range is
    not entirely file-backed in one section». Directory counts read 0/bogus while payloads
    physically exist (independent walker: DataDirectory entry region all-zeros; root-header counts
    0 though entries with the directory bit sit at file 0x12000) ⇒ do NOT promise RT_ICON
    inventories from REA here; use payload tools `work/aida64/logs/{png-dump.py,atlas-scan.py,
    dib-hunt.py,atlas-analyze.py,hex-hunt.py,hex-attribute.py}` → `work/aida64/out/atlas/`
    (icons10 163 PNG 256px + 1139 payload-length-validated 32-bpp BGRA DIBs
    16/20/24/32/40/48/60/64/72/80/96/256; icons2k 372 4-/8-bit 16/32; uires 410 + uireshd 386 PNGs
    = 1.5×/2×/3×/4× pyramids). Capture-match: tree-glyph crop x=29..44,y=83..97 over #626262 =
    16×16 BGRA-alpha DIB MAD 8.64 (runner-up 29.35 ⇒ atlas source identified); 29×26 ribbon blob
    MAD 39.53 unresolved. Gradient/ramp/dither hexes #3E4773/#473311/#FEA61E/#A87420/#828790/
    #848494 appear only at coincidence rate file_len/2**24 inside compressed streams (UPX1,
    deflate IDAT, .reloc tables) and the inflated `aida64.dat` has ZERO ⇒ renderer-computed colors
    ⇒ crate models endpoints + `GRADIENT_DITHER_AMPLITUDE` 4; no skin bitmap exists to embed.
    Caveat: when the compose image rebuilds, `make pe-resources` stdout gets docker bake-progress
    prepended → strip to the first `{` before jq or pre-build the image.

## Измеренные факты этой среды (host: 11 960 MB RAM + 16 GB swap, 6 cores, rootless Docker)

- cold Ghidra-сессия на distinct query (малый PE): ~18–31 s; на 51 MB unpacked AIDA64:
  **analyze 993 206 ms**, **function(entry) 1 083 627 ms**; warm snapshot replay того же query:
  **477.7 ms** (~2078× faster); packed-variant analyze: 29 766 ms.
- AIDA64 target (portable zip `AIDA64_Extreme.zip`, 65 508 373 B, sha256 `86ef98251edca384af1e09c1f818ef67617d7f114fe9635e5ad9d4812d32aae0`):
  распакованный каталог `work/aida64/AIDA64/`; `aida64.exe` 16 968 296 B = PE32+ x86-64 GUI, sha256
  `6265272bb5341d8056baab21116b419493ab19482df7eb7cf7c7a026b44ea337`, **UPX-packed** (upx 4.2.4:
  16 968 296 → 50 997 864 B, ratio 33.27%, `win6pe`); unpacked sha256 `588a844079e9a1f93d021c761531f7dbd589576c79f0482a53549c1538df0820`.
  Packed overview: procedures 21 / segments 4 / strings 74 (`UPX0` present) → unpacked:
  **procedures 51 971 / segments 12 / strings 98 359**. `aida64.ini` = `[Generic] NoRegistry=1 / NetUpdateFreq=0`
  (cured portable). Companion artefacts: `aida_bench64.dll` PE32+ console x86-64, `aida_uires.dll` PE32+ DLL,
  `kerneld-x64.sys`/`kerneld-v64.sys`/`kerneld-x64c.sys` = PE32+ **(native)** kernel drivers (RE targets),
  `aida64.dat` = zlib-compressed.
- REA limits honored: PE `inspect-native-load-image` attestation = `unsupported`; псевдокод ≠ original source;
  metadata edits ephemeral (Ghidra ephemeral DB); `pe-resources` provider = `rea-pe-resources` (engine-free,
  ~20-24 ms, resource `type` field не string → не `ascii_downcase`'ить в jq); image 2.73 GB;
  available RAM dips to ~2.5 GB during a 51 MB PE auto-analysis (with `MEM=6g MAXMEM=2G`) + swap usage —
  raise `MEM`/lower `MAXMEM` if OOM/`cleanup_incomplete`.
- Out of scope по решениям пользователя: MCP/agent-тотология (stdio-only), GUI/noVNC, динамика AIDA64
  (Wine/Windows-VM/remote `frida-server`), extended tools (pwntools, gdb+pwndbg, JADX, apktool, tshark,
  sigrok-cli, binwalk/unblob, WABT, mitmdump, adb, Chrome), IDA/Hopper/JEB, Ghidra NativeAOT,
  source-build REA из `./rea`, REA fixture lanes, multi-arch.

## Где читать первоисточники (не пересказывать)

`rea/docs/cli.md` (команды, snapshot-семантика, `--json`/token flags, jq-пайплайны),
`rea/docs/installation.md` (env-таблица Ghidra/Java, timeouts, analyzers/seeds, memory knobs),
`rea/docs/binary-diagnostics.md`, `rea/SECURITY.md` (Ghidra not sandboxed),
`rea/src/config/environment.ts` (zod env schema), `rea/src/ghidra/GhidraInstallation.ts` +
`GhidraJavaLaunch.ts` (что именно проверяется/запускается), `rea/AGENTS.md` (правила REA-репо).

## Как проверять изменения (check ladder)

1. `make -n build doctor` (dry syntax) → 2. `make pin-check` (digests live) →
2. `docker compose config --quiet` (schema) → 3. `make build` (runs core gate: versions+prereqs+providers+
   scoped doctor+ELF cold/warm) → 4. `make selftest` (full contract incl. PE/UPX + boundaries) →
5. engine-free targets (`pe-resources`, `bytes`, `offset`, `analysis-view` incl. inverted failure) →
6. cold/warm analysis runs на target'е как background jobs + `rg` на `"durationMs":` →
7. `git -C rea status --porcelain` empty, `make chown` в конце.
8. при правках `ui/aida64-theme/`: `CARGO_HOME=/home/opencode/.cargo cargo build && cargo test &&
   cargo clippy --all-targets && cargo fmt --check` (31 unit incl. compose+icon tests + 12 CPU-raster
   + 12 GPU + 2 doctests = 57 green, clippy warning-free; logs в `work/aida64/logs/ui-*.err`). GPU-ожидания правятся только
   после свежего dump'а (`cargo test --test gpu_snapshot gpu_debug -- --nocapture` → prints bbox/
   runs/counts + `target/gpu-debug*.png`; composition expectations from `gpu_composition_dump`); golden-сверка `python3 tools/model_vs_capture.py` →
   `work/aida64/out/ui-golden-vs-captures.txt`. Expectations from the spec-§5 measurements or a
   fresh dump only.
Expectations must come from measured behavior/docs, never from a guess; failures must stay explainable
(suject error JSON written to the target's stdout file is the primary evidence).
