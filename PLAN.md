# PLAN — Docker RE toolkit (`nutcracker`), первый target: AIDA64

Репозиторийный контракт для одной возобновляемой цели. **Продуктовая цель (зафиксирована user'ом
2026-10-15): полный клон AIDA64 — native Linux; Windows-платформа вне scope** (ни Wine, ни VM, ни
Windows-only API — product-подсистемы строятся на Linux-источниках; Windows-артефакты остаются
in-scope как RE-инструмент). **Parity = два равных, одинаково обязательных контура** (top-of-file
контракт в `AGENTS.md`): (1) **visual/frontend parity** — пиксельная геометрия/палитра/типографика
UI AIDA64; (2) **feature parity** — тот же user-visible функционал (inventory/sensors/benchmarks/
log-graph/licensing) на Linux-источниках. RE-контракт ниже (7 критериев + этапы B/C) — **substrate**
этой цели, а не self-цель. Closing-claim на любом контуре без evidence-артефакта (measurement/spec/
test) = fail-closed. Реализация substrate завершена и измерена (см. Evidence log); `./rea`
(REA v6.4.0 checkout) остаётся reference-кодом и **не изменялся**. Проверка прогресса — только по
checkpoint-лестнице (§Checkpoints) и ledger'у (§Feature-parity ledger).

## Контракт: 7 критериев done (все выполнены, проверено в этой среде)

| # | Критерий | Статус | Доказательство |
| - | -------- | ------ | -------- |
| 1 | `docker compose -f docker/compose.yaml build` exit 0; digest-проверки Node/JDK/Ghidra + version-gate Ghidra проваливают сборку при mismatch | DONE | образ `nutcracker-rea:latest` собран; bootstrap-ветка `GHIDRA_SHA256=PENDING_VERIFY_AND_PIN` проверялась ранее (печатает digest и exit 1) |
| 2 | Build-time gate: scoped doctor healthy + providers-assertion внутри образа | DONE | `SELFTEST CORE PASSED` в логе сборки; 9/9 `scope_checks` ok (node/ghidra/platform/arch/installation/version/headless/native_decompiler/java) |
| 3 | Selftest: cold Ghidra analyze + warm snapshot replay быстрее | DONE | ELF `/bin/cat`: cold **18358 ms** → warm **1854 ms**; PE seed: cold ~26–31 s → warm **2220 ms** (assertion warm < cold) |
| 4 | AIDA64-контракт: `upx -d` + анализ packed vs unpacked (UPX-феномен) | DONE | packed: procedures **7**/segments **5** (UPX0/UPX1); unpacked: procedures **388**/segments **6** (.text); seed = UPX-PE с именем `aida64.exe` (`work/unpackdemo/`) |
| 5 | `./rea` untouched; новый код только в `docker/`, `Makefile`, `recipes/`, `README.md`, `.gitignore`, `work/.gitkeep` | DONE | `git -C rea status --porcelain` пустой |
| 6 | На хосте не нужны Node/Java/Ghidra/upx/jq — каждый рецепт = `docker compose run` (и GNU make) | DONE | `make doctor/providers/capabilities/bytes/offset/analysis-view` executes `rea`+`jq` внутри контейнера |
| 7 | MCP/GUI/extended-tools вне scope и зафиксированы | DONE | README + recipes + раздел «Out of scope» ниже |

## Checkpoints (проверяемые finish-lines)

Формат checkpoint'а: **goal → gate (конкретная команда/артефакт + ожидаемый измеренный результат) →
статус → блоки**. Статусы: `DONE` (gate выполнен, измерен), `PARTIAL`, `NOT-STARTED`,
`BLOCKED(<внешний input/право>)`, `OPEN (решение user'а)`. Статус меняется **только** fresh-доказательством
(лог/артефакт/exit code), а gate — только измеренным поведением или документом; «правдоподобно» не считается.

| CP | Goal | Gate (machine-verifiable) | Статус | Блоки/зависимости |
| -- | ---- | ------------------------ | ------ | ----------------- |
| CP-0 | RE-toolkit substrate (критерии 1–7 выше) + этапы B/C | `make selftest` exit 0 (`SELFTEST FULL PASSED`), build-gate `SELFTEST CORE PASSED`, `git -C rea status --porcelain` empty, `make pin-check` exit 0 | **DONE** | — |
| CP-1 | UI-контракт substrate: 57-test гейм + golden-сверка | `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check` = 57 green (31 unit + 12 CPU-raster + 12 GPU + 2 doctest), clippy 0 warnings, fmt clean; `python3 tools/model_vs_capture.py` exit 0 → **30 `[PASS]`** (`work/aida64/out/ui-golden-vs-captures.txt`) | **DONE** | GPU-ожидания правятся только fresh dump'ом |
| CP-2 | Контракт-doc-sync: цель + dual parity + CI/release-facts + checkpoint-лестница + ledger в `PLAN.md`/`README.md`/`AGENTS.md` | grep-gates: цель-строка («native Linux», «Windows … вне scope») и «Parity» присутствуют в `PLAN.md` и `README.md`; заголовки `## Checkpoints` и `## Feature-parity ledger` присутствуют; ledger ≥ 6 строк, каждая с evidence-требованием; после push'а CI `success` + tag published | **DONE** (этот push) | self-verifiable: см. Evidence log (runs/releases) |
| CP-3 | Feature-contract извлечение: фича → Windows-механика → Linux-источник | артефакт `work/aida64/out/feature-contract.md` (host-side writes восстановлены — CP-15 DONE) с ≥ 8 feature-rows; каждая row ссылается на существующий артефакт (`test -f` по каждому cite); counts сверяются с `out/analyze-aida64.unpacked.exe.json` (procedures **51 971**, strings **98 359**) и `out/kerneld-x64.sys-driver-map.md` (pci-cfg 72 / phys-io 20 / porthw 14 / msr 6) | NOT-STARTED | нет write-blocker'а; требует `test -f` cross-check по citation'ам |
| CP-4 | Linux data-source readers (инвентарь: CPUID/PCI/hwmon/ports/DMI) = engine slice #1 | `cargo test --locked` green на fixture-based reader-тестах (fixtures repo-tracked) + measured host-inventory артефакт: `/sys/bus/pci/devices` = **10** (first = Intel `0x8086:0x1237`, `config`/`vendor`/`device` readable unprivileged), `/proc/cpuinfo` processors = **6**, `/sys/class/dmi/id/*` values readable (`product_name` = `KVM Server`), `/sys/class/hwmon` = **0 entries** (glob `cooling_device*` fail'ит today; an earlier probe saw 4 ⇒ re-measure), `/sys/class/thermal` = 0 zones | NOT-STARTED | MSR/физические порты/сенсоры require privileged-или-physical host: `/dev/cpu` absent (msr module не loaded), `/dev/port` = `crw-r----- root kmem`, `sudo` needs password, KVM guest ⇒ here только CPUID/PCI/DMI + fixture-лента |
| CP-5 | Реальная геометрия окна/statusbar + 2× HiDPI captures | новые capture-файлы в `work/aida64/incoming/` + `probe.py` output (dims/pHYs записан) + spec §updated + `cargo test` green с новыми asserts + golden rerun `[PASS]` ≥ 30 | **BLOCKED** | user-provided captures (Windows-рендер = measurement-инструмент, не product-runtime) |
| CP-6 | Chrome completeness: tab-strip/ribbon/statusbar composition | `src/compose.rs` даёт bands по capture-координатам + GPU asserts green по fresh dump'у + golden count не падает | **BLOCKED** | CP-5 |
| CP-7 | Log-list body rows через `egui_extras` Table | `Cargo.lock` содержит `egui_extras` 0.36.2 (measured: crate есть в offline-кеше `CARGO_HOME`, `--locked` resolves) + row-pitch измерен по CP-5 capture'ам + CPU/GPU asserts + golden row-count | **BLOCKED** | CP-5 |
| CP-8 | Шрифт/TTF + ClearType-френджи (#202074/#BCFFDE/#994C20 vs egui grayscale AA) | артефакт `work/aida64/out/font-metrics.md` (glyph metrics из capture probe'а) + тест с **quantified** Δ (ожидание из fresh dump'а, не guess) + решение (embed-TTF / accept-delta) зафиксировано в PLAN/AGENTS | NOT-STARTED | CP-5 + TTF-license решение user'а |
| CP-9 | Window run + renderer lane (wgpu сейчас, glow позже) | на display-capable хосте: `cargo run` + screenshots при PPM 1 и 2 + px-diff script против capture (численный Δ задокументирован); glow-вариант (`NativeOptions.renderer` Glow + feature) проходит тот же gate | **BLOCKED** | measured: `Xvfb`/`xvfb-run` absent на этом хосте; CI apt не содержит xvfb |
| CP-10 | Sensor graph/log-panel с реальными данными | measured graph-pitch 10.0 px (evidence `ui-golden-vs-captures.txt`) + data-driven rows из CP-4 readers + CPU/GPU asserts на значениях (fabricated-ink запрещён) | NOT-STARTED | CP-4, CP-7 |
| CP-11 | Inventory-дерево с реальными узлами | tree-ноды = CP-4 device list; selection-ring контракт остаётся green (`gpu_composition_bands_and_scrollbar` + `pixel_grid`) | NOT-STARTED | CP-4, CP-6 |
| CP-12 | Benchmarks parity (`aida_bench64.dll`: PE32+ console x86-64) | **user-supplied** baseline (Windows-run вне product-scope ⇒ числа из артефакта/документа) + Rust-реализация + tolerance-assert (Δ% измерен и задокументирован) | NOT-STARTED | baseline evidence от user'а |
| CP-13 | Licensing/registration UX | либо decision в §Out of scope, либо реализация с gate (license-file/ENV parsed + unit test) | **OPEN** | решение user'а |
| CP-14 | Release/CI discipline | каждый `master`-push: CI `success` + tag `v<run>` published (measured `v0.1`…`v0.4`); версия immutable (`0.$GITHUB_RUN_NUMBER`); `git -C rea status --porcelain` empty | **DONE** (continuous) | doc-only push'и тоже burn'ят version (см. AGENTS rule 12) |
| CP-15 | Write/ownership environment | gate: host-side write probe + container-mapping probe. **DONE (restored 2026-10-15 evening)**: `work/*` = `drwxrwxr-x 1003:1003`, `touch`/`rm` в `work/aida64/out` OK; container-uid-0 writes host-visible как `1003:1003 644` ⇒ evidence-артефакты writable с обеих сторон. (Ранее в тот же день: uid-0-owned + `make chown` EPERM = blocked, state (a)) | **DONE** | нет root-required work; **session-dependent** ⇒ мерить `stat -c %u:%g` + write-probe перед артефакт-writes |

## Feature-parity ledger (открыт 2026-10-15; пустой, но явный)

Формат: feature-семья → Windows-механика (RE-evidence, measured) → Linux-источник (product) → статус →
CP → **evidence-требование**. `DONE` только с evidence-артефактом; `NOT-STARTED` rows не дают права
заявлять parity.

| Feature-семья | Windows-механика (evidence) | Linux-источник (product) | Статус | CP | Evidence-требование |
| ------------- | --------------------------- | ------------------------ | ------ | -- | ------------------- |
| CPU inventory / CPUID + MSR | MSR allowlists: read **288** slots @`0x1e140`, write **84** @`0x1e5c0` (`out/bytes-kerneld-x64.sys-0x1e140.json`); CPUID-strings в `out/search-aida64.unpacked.exe.json` | `cpuid` instruction, `/proc/cpuinfo`, `/dev/cpu/N/msr` | NOT-STARTED | CP-4 | measured MSR-supported list + fixture-тесты (+ live-read на host с msr) |
| PCI/PCIe inventory + config space | pci-cfg **72** IOCTL'а (`out/ioctl-map-kerneld-x64.sys.tsv`); helpers CF8/CFC `FUN_00011358`/`FUN_00011474` | `/sys/bus/pci/devices/*/{config,vendor,device}` | NOT-STARTED | CP-4 | fixture + live-host read proof (10 devices measured here) |
| Physical I/O + port HW | phys-io **20**, porthw **14** IOCTL'а; port deny-list `FUN_000122d8` | `iopl(3)`/`ioperm` (требует CAP_SYS_IO) | NOT-STARTED | CP-4 | measured port-список + perms-probe output |
| Sensor monitoring (temps/voltages/fans) | UI-shape измерен (captures/spec); Windows-backend не RE'd | `/sys/class/hwmon/*`, `/sys/class/thermal/*` | NOT-STARTED | CP-4/CP-10 | live-host sensor inventory (здесь hwmon-dirs = 0) + UI-маппинг |
| Log/graph panel (CompuScope) | geometry measured: header row 24 px, graph pitch 10.0 px (`ui-golden-vs-captures.txt`), tree row 18 px | egui paint (DONE geometry) + CP-4 readers | **PARTIAL** (geometry/chrome only) | CP-10/CP-7 | data-driven asserts + measured row pitch |
| Tree/navigation | measured ring **81×18**, row 18 px, step 19 (`ui-golden-vs-captures.txt`, spec §5) | `add_nested` exact depth (DONE geometry) | **PARTIAL** | CP-11 | реальный node-список из CP-4 |
| Benchmarks (CPU/FP/memory bandwidth, SST) | `aida_bench64.dll` = PE32+ console x86-64 (PE-facts measured, `pe-resources-*.json`) | Rust bench-реализации | NOT-STARTED | CP-12 | user-supplied baseline + tolerance |
| Reporting/export + licensing | `licens*` strings hit'ы в `out/search-aida64.unpacked.exe.json` | TBD (product-решение) | **OPEN** | CP-13 | decision + tests |

## Artefacts и entry points

| Файл | Роль |
| ---- | ---- |
| `docker/Dockerfile` | единый источник pin'ов (`ARG`-блок) + слои: base/apt → Node → JDK → Ghidra → `rea` → selftest-gate |
| `docker/compose.yaml` | один сервис `rea` (`platform: linux/amd64`, `mem_limit: ${REA_MEM:-4GiB}`, bind `../work:/work`, volume `rea_home:/root/.rea`, без портов) |
| `docker/selftest.sh` | build-gate (`core`) и полный self-proof (`full`: ELF + Windows-PE/UPX + Evidence-boundary + analysis-view + engine-free JS) |
| `docker/pin.sh` | `make pin` / `make pin-check`: сверка и обновление digest-pins из upstream (Node SHASUMS / Adoptium `.sha256.txt` / Ghidra stream) |
| `Makefile` | все рецепты: `build pin pin-check selftest selftest-core doctor providers capabilities shell unpack analyze decompile function search trace xrefs instructions bytes offset pe-resources load-image managed-artifact analysis-view evidence chown clean` |
| `recipes/00-build-selftest.md` | хост-требования, сборка, selftest, ownership/rootless-заметка, image size |
| `recipes/01-aida64-static.md` | AIDA64-поток: unpack → analyze → search/function → dossier → native-view paging; tuning knobs |
| `recipes/02-snapshots-evidence.md` | что cached vs всегда-cold, snapshot layout, record-vs-bundle граница, token economy |
| `.gitignore`, `work/.gitkeep` | target-бинарники и Evidence не коммитятся |

## Ключевые facts (pinned + measured)

- Pinned: `rea-agents@6.4.0`, Node **24.18.0** `55aa7153f9d88f28d765fcdad5ae6945b5c0f98a36881703817e4c450fa76742`,
  JDK 21 **21.0.12.1+1** `ce79869e1307ed8ee1e2baa86a412b1eb5b75d10a01006d788a6f968bcfaee94`,
  Ghidra **12.1.4** (`ghidra_12.1.4_PUBLIC_20260921.zip`) `ddac49f903da9d5bac833e5cc79395098b9c33cfd3279be5f31bd00387d2d4db`.
  `make pin-check` → «All digest pins match upstream», exit 0. Ghidra-архив check-артефактов не публикует →
  digest получен stream-хешем официального GitHub-asset и зафиксирован в `ARG`.
- Ghidra: `application.properties` = `application.version=12.1.4`, `application.java.min=21` (max пустой) →
  JDK 21 admitted; REA проверяет `support/analyzeHeadless` и native decompiler `os/linux_x86_64/decompile`
  по executable-заголовку (doctor detail: «reports elf x86_64, which matches linux/x64»).
- Image size: **2.73 GB** (Ghidra+JDK доминируют; extended-tools deliberate off).
- Providers в core-образе: `ghidra available`, `hopper unavailable (executable_missing)`, `ida unavailable (not_configured → REA_IDA_MCP_CONFIG)`; capabilities count **18**.
- Производительность (measured, Ghidra-сессия = отдельный процесс на distinct query):
  cold **~18–31 s**; warm snapshot replay **~0.19–2.2 s**; engine-free built-ins **~24–90 ms**
  (`inspect-pe-resources` 20205 B, `inspect-managed-artifact`, `analyze-javascript-application` 135463 B, `inspect-analysis-view`);
  cached-виды: `analyze`/`search`/`function`/`trace`; **`decompile`/`instructions`/`xrefs`/`load-image` остаются provider-cold**
  (см. `rea/src/application/binary/AnalysisSnapshotCache.ts` и `rea/docs/cli.md:177,185-189`);
  `read-bytes`/`address-to-file-offset` не принимают `--snapshot` (measured: `Unknown flag: --snapshot`).
- Ghidra-архив сам содержит Windows-PE артефакты (`docs/GhidraClass/ExerciseFiles/WinhelloCPP/WinHelloCPP.exe`) →
  PE/UPX-контракт проверяется **без downloads и без host-инсталляций**; UPX 4.2.4 round-trip не byte-identical
  (110592 → 62464 → 110592 B, ratios 56.48%) → сравниваем содержимое анализа, не digests.
- Ownership/idmap (measured; **меняется между daemon-сессиями** ⇒ мерить `stat -c %u:%g`): Docker здесь
  **rootless**, host-uid = 1003. Два состояния, измеренные 2026-10-15: **(a) uid-blocked** — container
  uid 0 ↔ host uid **297 610**, host-uid вне idmap-range ⇒ host-side writes в `work/*` (uid-0-owned)
  **невозможны**, `make chown` → `chown: changing ownership of '/work': Operation not permitted`,
  artifact/log-записи шли container-side (`/work/<target>/…`) или в user-owned dirs. **(b) restored
  (вечером, после operator-fix)** — `work/*` = `drwxrwxr-x 1003:1003`, host-side writes проверены
  (`touch`+`rm` в `work/aida64/out` OK) и файл, созданный container-uid 0, host видит как
  `1003:1003 644` (probe `/work/_map-probe`) ⇒ evidence-артефакты writable с обеих сторон. `sudo` на
  хосте отсутствует («a password is required») ⇒ ownership-fix только operator-side. in-container
  `chown -R 1003:1003` не «чинит» ownership, а re-ownership'ит host-1003-файлы на mapped-uid
  (measured в (a)) ⇒ повторять его нельзя.
- REA-ограничения, учтённые в рецептах: PE load-image attestation `unsupported`; pseudocode ≠ оригинальный source;
  metadata-annotации ephemeral (discarded вместе с ephemeral project); CLI-`--json` = **один Evidence record**,
  bundles — только из `export_evidence_bundle`/saved bundle (иначе `evidence-export`/`compare` fail closed);
  `inspect-analysis-view` требует positional `inputJson` (stdin не читает) и принимает `view:{kind:"native",facet:...}`
  только для records с parent `analyze_function` / `analyze_javascript_application` / `inspect_binary_layout`;
  npm 6.4.0 CLI **не** имеет `inspect-go-binary` (main-only); пустые значения `REA_*` env reject'ятся zod-схемой
  (`rea/src/config/environment.ts`) → Makefile подаёт `-e` только непустые.

## Как продолжить (следующие ступени, по порядку)

1. Положить настоящий AIDA64 build в `work/aida64/aida64.exe` (license-hold) и пройти
   `make unpack` → `make analyze SNAPSHOT=aida64.json` → `make search QUERY=...` → `make function NAME=entry`
   (рецепт 01). При необходимости tuning: `TIMEOUT=900000 MAXMEM=1G DISABLED_ANALYZERS=WindowsResourceReference SEED_FILE=/work/aida64/seeds.tsv`.
2. Если нужен human-GUI/динамика/extended-tools — это отдельные image-variants (см. Out of scope);
   текущий контракт их намеренно не включает.
3. При bump REA/Ghidra/Node/JDK: `make pin` → `make build` → `make selftest` (gate должен остаться зелёным).

## Расширение контракта (новые этапы после 7 базовых критериев; всё измерено)

### Этап B — native-драйверы AIDA64 (`kerneld-*.sys`) — DONE (static RE)

| Факт | Значение |
| ---- | -------- |
| три distinct-драйвера | `kerneld-x64.sys` 81 304 B `ae6b4853e9c2d433d0955627edaba8fbb88bef0cafbb1f9df87a796a8eab0069`; `kerneld-x64c.sys` 68 376 B `eba32338…794b`; `kerneld-v64.sys` 34 648 B `27375351…f888f` (не клоны) |
| RT_VERSION (engine-free `pe-resources`) | File/ProductVersion **3.0.1022.0**, InternalName `LLKD`, OriginalFilename `kerneld.x64`, FinalWire Ltd., «AIDA64 Kernel Driver» |
| DriverEntry | INIT-stub RVA 0x12064 → loaded `0x22064` → `FUN_00012808`: `IoCreateDevice` DeviceType **0x8011**, `\Device\AIDA64Driver` + symbolic `\DosDevices\AIDA64Driver`; dispatch-writes +0xE0/+0x80/+0x70 → `0x12a84`, +0x68 → `0x1d308` (teardown) |
| IOCTL-карта | `0x80112040..0x801122a0` шаг 4 → func **16..168 = 153 handlers** (contiguous); классы: pci-cfg 72 (61 с phys-io), phys-io 20, porthw 14, msr 6, irp-relay 2 (`0x801120bc`, `0x801120d8`), timing/IRQL 2 (`0x801120b0` PIT+TSC), stub 39; gate `*pcVar26 != 0x0e` = `IRP_MJ_DEVICE_CONTROL`; одна точка completions (`IofCompleteRequest` + `FUN_0001d550`) |
| хелперы (dossiers) | `FUN_00011358` = PCI cfg dword (CF8/CFC или `HalGetBusDataByOffset`), `FUN_00011474` = byte/word (second CF8 fetch), `FUN_00011308` = aligned dword copy, `FUN_000110fc` = `MmMapIoSpace` dword write, `FUN_000122d8` = port deny-list (0x000-0F, 0x20-21, 0x60/64, 0x70-71, 0x80-8F, 0xA0-A1, 0xC0-DF, 0x170-17F, 0x1F0-1FF, 0x376-7, 0x3E0-EF, 0x3F6-7) |
| MSR allowlists (engine-free `bytes`) | read **288** slots @`0x1e140` (287 nonzero/unique), write **84** @`0x1e5c0` (contiguous); incl. architectural, AMD 0xc0000080-89/0xc0010000-11023, vendor-namespaced 0x8086801x/3x |
| chipset-таблица | 252 dwords `low16 == 0x8086` (device ids 0x1c42–0x3b0x …) → `work/aida64/logs/kd-ids-8086.txt` |
| NTSTATUS-словарь драйвера | ровно 3 кода: `0xc000000d` ×339, `0xc000003e` ×88, `0xc00000bb` ×16 (имена сверены через codex web с wine-mirror `ntstatus.h`) |
| cost | cold Ghidra ~78–80 s/query: dispatcher dossier **180 892 ms**, teardown **154 822 ms**, PCI/registry **162 279 ms**, 5 helper dossiers 164 519 / 171 064 / 178 064 / 163 214 / 156 439 ms; engine-free bytes-read 89 661 ms (container overhead, без Ghidra-движка) |
| артефакты | `work/aida64/out/kerneld-x64.sys-driver-map.md`, `work/aida64/out/ioctl-map-kerneld-x64.sys.tsv`, `logs/kd-map.py`/`kd-report.py`/`kd4.sh`/`kd5.sh`, `logs/f12a84.pseudo.txt` (264 886 B) / `.asm.txt` |
| открыто | `FUN_000110ac` (read-symmetрic of 110fc) — inference by call-site shape; input-struct layout для `0x801120bc`/`0x801120d8`/`0x80112248-250`; dossiers `FUN_00011274`/`11674`/`11700`/`1114c`; opcode-diff vs `x64c`/`v64` |

### Этап C — пиксель-тема AIDA64 для Linux (egui 0.36.2) — DONE для theme-краста

- 4 capture'а (`ksnip_20261014-003*.tar.gz` → `work/aida64/incoming/`, sha256 `c528f29fc6ac…e62c`):
  926×668 / 926×665 (main window v8.40.8500), 541×518 (Cache & Memory Benchmark), 776×593
  (System Stability Test); RGBA8, `pHYs` 96.012 → **PPM = 1.0**, alpha 255 (no shadows).
  Спецификация с measurements: `work/aida64/out/ksnip-20261014-003-ui-spec.md` (§5 константы/палитра,
  §6 egui-mapping, §7 gaps); инструмент измерений `work/aida64/incoming/probe.py`.
- `ui/aida64-theme` (`aida64-ui-theme`, dep `egui = "=0.36.2"`): `palette.rs`/`metrics.rs` (каждый
  const с provenance-комментарием), `style.rs` (`aida64_style()`/`apply`/region-патчи/frames),
  `paint.rs` (device-px `fill_rect` helpers), `tests/pixel_grid.rs` (CPU-растровый гейм: egui
  real-queued shapes → raster → pixel asserts при PPM 1/2/3), `tests/gpu_snapshot.rs` (raster через
  реальный renderer), `tools/model_vs_capture.py` (golden-сверка констант с capture'ами).
- Gates GREEN (**57 tests**): `cargo build` exit 0; `cargo test` = 31 unit (incl. widgets/compose/icon tests) + 12 CPU-raster + 12 GPU
  raster + 2 doctest, все pass; `cargo clippy --all-targets` 0 warnings; `cargo fmt --check` 0 diff
  (logs `work/aida64/logs/ui-build-8|ui-test-33|ui-test-34|ui-clippy-11|ui-fmt-9.err`, exit 0).
  Deps resolvable из `CARGO_HOME=/home/opencode/.cargo` (registry-источники egui/eframe/egui_glow/
  wgpu/egui_kittest распакованы); GPU-dev-путь требует `image` c `default-features = false` (дефолтный
  `default-formats` тянет `exr`, которого нет в локальном index-кеше — `ui-test-17.err`); network egress
  работает (`ui-test-18/19`); `cargo clippy --fix` требует `--allow-dirty` при dirty tree (measured при
  0-commit history; сегодня tree clean и `.git` host-writable ⇒ fix-lane работает ordinary'но —
  мерить `git status --porcelain` перед ней).
- GPU-lane (`tests/gpu_snapshot.rs`, 9 tests) рисует через headless **lavapipe**
  (`/usr/share/vulkan/icd.d/lvp_icd.json`); `RendererOptions::PREDICTABLE` → determinism запечатлён
  тестом `gpu_raster_is_reproducible_run_to_run` (two independent harnesses → identical images).
  Measured harness-факты: egui reserves **8-point content inset** (bbox `(8,8)-(191,191)` на 200-pt
  canvas @PPM 1, 16 px @PPM 2); `CentralPanel`-painter clipped к content rect; readback clear color =
  прозрачный `#000000/00`; every painted px = exact measured hex с zero AA (row/col counts точные,
  соседние rows/cols 0); rect, который 1 device px в *обоих* измерениях, tessellates в degenerate
  zero-width core и рендерится как **0 px** (`ui-test-25.out` bounds `[[10.5 13.5] - [10.5 13.5]]`) →
  per-device-pixel dither остаётся CPU-kонтрактом. Concurrent lavapipe devices abort'ят процесс
  (SIGSEGV, `ui-test-29.err`) → `render_scene` держит module `Mutex` (plain `cargo test` green);
  fallback `--test-threads=1`. `gpu_debug_dump` печатает bbox/row-cols-runs/counts и пишет
  `target/gpu-debug*.png` — GPU-ожидания правятся exclusivно по dump'у (все 5 expectation-ошибок
  этого прохода исправлены именно так: ring/panel-tone row conflation, thumb rows 40..=79,
  `STROKE_SELECTION` interior = 2 px, off-line grid px, dither 1×1 cull).
- Textured lane (measured, `ui-test-41..64`): kittest `Harness::render()` не применяет
  `TexturesDelta`, egui drains alloc-deltas во время `run_ui` (`load_texture` = manager-alloc only,
  `context.rs:2390-2410`) ⇒ на readback manager И last-pass output deltas пусты (`ui-test-63.out`
  `delta set keys []`), AND `TextureManager::Default` pre-allocates 1×1 WHITE at `Managed(0)`
  (`textures.rs:17-18`), который fills sample'ят как `WHITE_UV` ⇒ без manual uploads в kittest-lane
  NOTHING textured rasterizes (fonts/иконки невидимы — `ui-test-46..52`). Fix =
  `render_scene_with_textures`: drain `ctx.tex_manager().write().take_delta()` (Arc-bind против
  E0716) + uploads `ImageDelta::full` WHITE@default + crate-stashed glyphs via
  `icon_texture_uploads()` (`ICON_TEX_SEQ` unique names ⇒ каждый invoke allocates+stashes;
  `build_ui` settle-passes ≥5 ⇒ "last-pass" wiring-гейты ненадёжны) before `Renderer::render` —
  eframe-эквивалент. Composition dump (`gpu_composition_dump`) + bands asserts
  (`gpu_composition_bands_and_scrollbar`) green по свежему dump'у `ui-test-43.out`
  (harness-inset 8 ⇒ absolute = capture+8: titlebar rows (8,37), toolbar (38,85), hairline row 86
  x=0..909, divider col 210, trough 189..=205 + gutter 206..=209, sel-fill rows (88,103), ring
  rows (87,104) + left col 35 = capture 27+8, arrow ink (94,96)+(908,910)); icon texel-perfect
  contract green (`ui-test-63/64.out`: origin (37,88), ink_texels=202, opaque_bad=0, asset alpha
  histogram {255:202, 0:54}, partial/delta counters 0). Font-atlas в offscreen-lane НЕ delivered
  ⇒ text-ink asserts там нет; eframe unaffected; fill-only tests остались на `render_scene` ✓.
- egui 0.36.2 facts, зафиксированные в AGENTS.md rule 11 (measured при сборке гейма): нет
  `TopBottomPanel`/`Tree`/`TabBar`/`Table`; scale = `zoom_factor × ViewportInfo.native_pixels_per_point`
  (measured drift 2.0 + `set_pixels_per_point(2.0)` = 4.0); `TexturesDelta::drop` panic'ует на
  unapplied deltas; `SELECTED_CLASS` в `widget_style.rs` перекрашивает текст selected-виджета
  (измеренный AIDA64 selection сохраняет `#FFFFFF` → tree selection рисуется вручную);
  `NumberFormatter` `PartialEq` = `Arc::ptr_eq` → whole-`Style` assert_eq невозможен.
- Open blockers pixel-perfect (measured gaps): точный шрифт/TTF + ClearType-френджи (#202074/#BCFFDE/#994C20 vs
  egui grayscale AA), реальная геометрия окна/statusbar/2× HiDPI captures (194-px panel & 81×18 box —
  crop-инстансы), unmeasured states: hover/pressed/unchecked/disabled text, log-list body row pitch
  + striping, Copy/Latency column bounds, axis-label цвета, value-box x-extents, toolbar semantics,
  locale strings. CLOSED by measurement this cycle: icon-atlas (payload-extracted +
  capture-matched, см. step 1) и img3 gradient/dither model (hexes renderer-computed, см.
  AGENTS.md rule 11 «FinalWire resource dialect» ⇒ crate воспроизводит endpoints + amplitude).
### Следующие ступени (по порядку)

1. Icon-atlas — DONE via payload extraction (resource tree unreadable: REA 0 resources +
   bogus directory counts, AGENTS rule 11): `work/aida64/logs/png-dump.py` (chunk-chain + CRC,
   IEND len 0 legit, `im.bits` absent ⇒ depth derived) + `work/aida64/logs/dib-hunt.py`
   (payload-length consistency; dialect: `biSizeOfImage` stores XOR-image size only, tolerance
   `size_image ∈ {0, xor, xor+mask}`; `biSize==40` false-positives are IMAGE_RESOURCE_DIRECTORY_
   ENTRY records e.g. `2800000068320080 2900000080320080 …`) → `work/aida64/out/atlas/`:
   icons10 32-bpp BGRA 16/20/24/32/40/48/60/64/72/80/96/256 px (1139 DIBs + 163 PNG 256) +
   icons2k 372 glyph DIBs 16/32 4-/8-bit; uires 410 + uireshd 386 PNGs покрывают 1×…4× пирамиды ✓.
   Capture-match evidence (`out/atlas-analysis.md`): tree-glyph crop (29..44, 83..97) = 16×16
   BGRA-alpha DIB MAD 8.64 vs runner-up 29.35 ✓; 29×26 ribbon blob MAD 39.53 unresolved ✓.
2. Golden-pixel DONE: `ui/aida64-theme/tools/model_vs_capture.py` (PIL-only, константы читает из
   `src/palette.rs`/`src/metrics.rs`, поэтому не может drift'нуть) → **30 PASS**, exit 0, evidence
   `work/aida64/out/ui-golden-vs-captures.txt`: palette occurrence 65/65 consts; per-image fringe
   attribution (img1 `[47,41,40]` selection-only, img2 `[972,783,854]` panel, img4 `[779,654,732]` +
   `[103,74,92]` button, img3 0 = `FRINGE_COUNT_IMG3`); band counts (origins img1 (8,3) / img2 (6,2)):
   titlebar 25 565/27 780 exact `#000000`, toolbar 39 770/44 448 `#202020`, wrong-surface budget ≤4 px
   (measured `#626262`×2); rhythm first-non-black row 33/32 = `TITLEBAR_H`, единственный dither-row
   81/80 (364/357 px); selection ring ровно **81×18** (left-ring vertical run 18 px, interior
   `#626262` 784 px — label-текст поверх fill); scrollbar slot x=179..195 width 17, trough rows 80..662,
   thumb cols 15, arrow `#676767` 42 px, внутри trough-span ни одного model-цвета; img4 header-группа
   rows 49..72 = `HEADER_ROW_H` 24; img4 graph `#008000` pitch ровно 10.0 на 156/178 rows; 6 квадратиков
   13×13 `#D2D2D2` (из 7 — AA скрывает контуры).
3. Custom widgets — DONE (geometry only, `src/widgets.rs`, measured egui 0.36.2 API):
   `paint_tree_row`/`tree_row_rects` (row 18 px, step via `Ui::indent` + spacing 19, selection box
   = 5 crisp fill-rects, label ink остаётся #FFFFFF в обход `SELECTED_CLASS`), chevron/icon slots
   зарезервированы (glyph ink unmeasured), `tab_strip`+`paint_tab_separators` (19 px, active
   отличается ТОЛЬКО пропущенным 1-px hairline), `paint_aida64_scrollbar` (trough/thumb/arrows),
   `paint_log_header_strip` (24 px #191919, один fill; body rows unmeasured). Gate 57 green
   (31 unit incl. widget/compose/icon тесты + 12 CPU-raster + 12 GPU + 2 doctest). Icon wiring
   DONE (`src/widgets.rs`): `IconPixels` + `paint_icon_rgba` (NEAREST texel-маппинг @PPM 1;
   `load_texture`/`TextureId` верифицированы компилятором+GPU-rasterem) + texel-perfect GPU
   контракт по winner-asset (`aida_icons10.dll_dib_16x16_32bpp_comp0_pal0_BGRA-alpha_92.png`,
   origin (37,88), opaque_bad=0, `ui-test-63/64`) + runtime-load из `work/aida64/out/atlas/`
   (binaries не коммитить, rule 10; golden atlas-glyph-match MAD 1.02 claim identifies source).
   Open: log-list body rows через `egui_extras` Table (pitch unmeasured).
4. Chrome composition — DONE (`src/compose.rs` + GPU asserts): `paint_aida64_chrome`
   (window_margin ZERO + spacing.indent=TREE_INDENT re-patch per pass (kittest style-reset
   measured); bands/hairline/divider/scrollbar-slot + TreeItem-rows via `add_nested` exact-depth;
   `ChromeRects` = painted ink rects: tree [8,202)×[79..), central [203,926)×[79..), divider col
   202, hairline row 78; measured panel semantics в AGENTS rule 11: `exact_size` incl. margins
   panel.rs:405, `response.rect` = reservation-from-cursor, window_fill == capture gutter). GPU
   composition dump/bands green по свежему dump'у `ui-test-43.out`; CPU compose unit tests green;
   thumb extent unmeasured ⇒ Option documented (no fabricated ink).
5. eframe shell — DONE-partial: гейты build+clippy+fmt exit 0 first-try (ui-shell-build-1, ui-shell-clippy-1 warnings=0, ui-shell-fmt-2); App trait method `ui` NOT `update` (lib.rs:47) compile-verified; runtime winner-glyph load wired; viewport 926×668 instance compiles; window run = display backlog. Original tail:  (wgpu сейчас, `egui_glow` позже по плану пользователя) + offscreen snapshot-kонтракт.

## Out of scope (решения пользователя, зафиксированы)

**Windows как product-платформа** (клон = native Linux; Wine/Windows-VM/remote `frida-server`
допустимы только как RE/measurement-инструмент, не как product-runtime), MCP/agent-тотология
(stdio-only transport; CLI-рецепты выбраны), human GUI (Ghidra CodeBrowser/noVNC),
динамика AIDA64 (`rea` process-capture не умеет Windows host),
extended capability tools (pwntools/gdb+pwndbg/JADX/apktool/tshark/sigrok/binwalk+unblob/wabt/mitmdump/adb/Chrome),
IDA/Hopper/JEB providers, Ghidra NativeAOT extension, сборка REA из `./rea` checkout, fixture-lanes REA CI,
rootless/Docker profiles и multi-arch образ.

## Evidence log (measurement trail)

- build gate: cold 18358 ms → warm 1854 ms; token probe `--token-count` = **1965** tokens for the warm overview.
- full selftest (GREEN): packed procedures 7 / unpacked 388; entry `0x401e46`, callees `___tmainCRTStartup`, `___security_init_cookie`;
  function warm replay 2220 ms; decompile cold 31033 ms; `inspect-pe-resources` 20205 B; native facet view 7993 B vs dossier 20879 B;
  JS summary 9763 B vs record 135463 B; boundary сообщение записано в `work/selftest/analysis-record-boundary.txt`.
- AIDA64-shaped batch (`work/unpackdemo/`): packed {7,5,12} vs unpacked {388,6,256}; container paths `/work/unpackdemo/...`;
  host ownership восстановлен (`ls -ln` → 1003:1003).
- `make pin-check` → OK node/jdk/ghidra (exit 0). Image 2.73 GB. `git -C rea status --porcelain` пустой.
- UI-theme final gate: build Finished 1.78 s; `cargo test` = 51 green = 28 unit (incl 7
  `widgets.rs` geometry) + 12 CPU-raster (`tests/pixel_grid.rs`) + 9 GPU
  (`tests/gpu_snapshot.rs --test-threads=1`, headless lavapipe `RendererOptions::PREDICTABLE`)
  + 2 doctest; clippy 0 warnings; `cargo fmt --check` clean. Harness facts (measured): egui 0.36
  `Context` has **no fonts until the first `run_ui` pass** (`context.rs:1103/1121` panic
  "No fonts available until first call to Context::run()"), and that pass queues one
  `TexturesDelta` that panics on drop unless `FullOutput.textures_delta.clear()`
  (`epaint/textures.rs:337`) — widget tests prime with `run_ui(RawInput::default(), |_| {})`
  then clear. Logs `work/aida64/logs/ui-{build-9,test-35..37,clippy-12/13,fmt-10/11}.err`.
  Golden rerun `python3 tools/model_vs_capture.py` → exit 0, 29 `[PASS]`.
  Shell crate `ui/aida64-shell` compile gate first-try GREEN: cargo build exit 0 (ui-shell-build-1.err) + clippy --all-targets 0 warnings (ui-shell-clippy-1.err) + fmt --check exit 0 (ui-shell-fmt-2.err); eframe 0.36.2 `App` trait method = `ui(&mut self, ui, frame)` NOT `update` (doc-example lib.rs:47, trait via `pub use epi::*` :167) verified by compiler; default-features `wgpu` satisfies `run_native` cfg (:286); `ViewportBuilder::default().with_inner_size([926.0, 668.0])` compiles (img1-crop instance); runtime atlas load wired (AIDA64_ATLAS_DIR else manifest `../../work/aida64/out/atlas`, `into_rgba8`→`into_vec`, eprintln degradation when missing). Theme gate re-verified after icon-test doc-line fix: ui-test-66/clippy-18/fmt-16/17 exit 0, 57 green unchanged; golden stays 30 `[PASS]`.
- UI-theme extended gate (composition + icon-wiring era): `cargo build && cargo test && cargo
  clippy --all-targets && cargo fmt --check` = 57 green (31 unit incl. 2 compose + 1 icon, 12
  CPU-raster, 12 GPU, 2 doctests), clippy 0 warnings, fmt clean — logs
  `work/aida64/logs/ui-{build-12,test-65,clippy-17,fmt-15}.err` exit 0. Zero-textured-raster
  diagnosis chain (`ui-test-41..52.{out,err}`): kittest `render()` no-delta-apply → egui
  drain-during-run → white@Managed(0) dependency ⇒ textured lane needs explicit uploads;
  `ICON-DBG` measured icon_slot [37,87)-[53,105) (18 tall) → after vertical centering
  [37,88)-[53,104) (16×16 GPU-verified); asset histogram {a255:202, a0:54} texels; `ui-test-52`
  opaque_bad=256 (`got=[0,0,0,0]` everywhere) → `ui-test-63/64` `ink_texels=202 opaque_bad=0
  partial_count=0 max_rgb_delta=0 max_alpha_delta=0`. Golden rerun (ui-golden-11.out) exit 0,
  **30 [PASS]**, evidence `work/aida64/out/ui-golden-vs-captures.txt` refreshed.
- AIDA64 UI assets (payload-level, engine-free): `rea inspect-pe-resources` → `resources 0 /
  icon_groups 0 / directories null` на aida_uires/uireshd/icons10/icons2k/aida64.unpacked.exe
  (`out/pe-resources-*.json`, logs `pr-*.err`; packed aida64.exe → `invalid_request` «Resource
  range is not entirely file-backed in one section»); `logs/png-dump.py` → 410/386/163 PNGs;
  `logs/dib-hunt.py` → 1139 (icons10, 32-bpp) + 372 (icons2k, 4-/8-bit) DIBs с alpha via
  AND-mask/BGRA; `logs/atlas-analyze.py` → 866/960 PNGs partial-alpha, dark-skin hexes внутри
  illustration arts, hairline dither pair только в 352×396 (4×) arts, MAD-таблицы glyph-match;
  `logs/hex-attribute.py` → coincidence-rate attribution len/2**24 (evidence
  `out/hex-hunt.txt`, `out/hex-attribute.txt`, `out/atlas-analysis.md`).

## Verification lanes (CI-measured, 2026-10-14/15)

- CI = `.github/workflows/native-release.yml` (`metadata` → `checks` ∥ `native` → `publish`) на Rust
  **1.96.1** с `cargo fmt --check` + `cargo clippy --locked --all-targets -D warnings` +
  `cargo test --locked` на обоих UI-крастах; GPU-lane в CI работает (apt `mesa-vulkan-drivers` ⇒ lavapipe,
  `RUST_TEST_THREADS=1`), но CI пропускает ровно `gpu_composition_icon_texel_perfect` ⇒ **local-гейт
  57 тестов остаётся authority** для texel-perfect icon; shell-краст = compile-only pass (`cargo test`
  no-op) ⇒ window/feature-поведение CI НЕ проверяет, green CI ≠ parity-доказательство.
- Push на любую ветку ⇒ checks + `native`-артефакт; push в `master` ⇒ release `v<run-number>` (immutable,
  bump-only-by-push). Архив = stripped bin + README **без atlas-PNG** ⇒ runtime-деградация по
  `AIDA64_ATLAS_DIR` (measured release body: «atlas binaries are not distributed»).
- Host/git-ограничения **session-dependent** (measured оба состояния 2026-10-15): `AGENTS.md` rules 6/10.
  В uid-blocked state push шёл из user-owned копии `.git` по ssh, writes — только container-side.
  В writable state (измерен сегодня) `git commit`/`git fetch`/`git reset --hard origin/master`
  ordinary'ные ⇒ **перед** work'ом мерить `git status -sb` + `stat -c %u:%g work/aida64/out`, а
  pushed-состояние post-push проверять через Actions/release API (tags локальны только после fetch).

### Evidence log — checkpoint-период 2026-10-15 (measured)

- CP-2 doc-sync: commits `ee5a058` («docs(AGENTS): pin Linux-clone goal + dual … parity contract…») →
  `e50cfb6` (correction push-процедуры) на `master`; Actions runs `38774205839` и `38775631047` =
  `completed success`; releases `v0.2` (published 2026-10-15T17:05:18Z) и `v0.3` (17:15:46Z), каждый
  **Assets = 1** ⇒ doc-only push burn'ит immutable version ⇒ batch policy (AGENTS rule 12).
- CI cargo-lanes на обоих doc-commits = green (тот же `checks` job) ⇒ UI-гейты CP-1 freshly verified
  CI'ом, кроме `gpu_composition_icon_texel_perfect` (local-only по-прежнему; локальные latest числа:
  `ui-test-65/66` = 57 green, `ui-clippy-18`, `ui-fmt-17`, `ui-golden-11` = 30 `[PASS]`).
- Push-процедура (measured): host-uid 1003 `git commit` → `could not open '.git/COMMIT_EDITMSG':
  Permission denied`; `gh` contents-PUT → HTTP 403 «Resource not accessible by personal access token»;
  working path = `cp -r .git <user-owned-copy>` + `GIT_DIR=<copy> GIT_WORK_TREE=<repo> git add/commit/push`
  (ssh: `ssh -T git@github.com` → «Hi 0FL01!»); локальный HEAD тогда оставался `569ec8c`
  (drift documented, AGENTS rule 10; cleared позже в тот же день — см. подраздел ниже).
- Gate-числа среды (используются CP-4/CP-7/CP-9/CP-15; measurement при открытии checkpoints,
  2026-10-15 day): `/sys/bus/pci/devices` = **10**; `/proc/cpuinfo` processors = **6**;
  `/sys/class/hwmon/hwmon*` dirs = **0** (тот probe видел 4 `cooling_device*`, сегодня glob
  fail'ит ⇒ re-measure); `/dev/cpu` = **absent** (MSR-лента требует host с msr-module + perms);
  `Xvfb`/`xvfb-run` = **not found** (CP-9 blocked); `upx` на host absent (in-image);
  `egui_extras-0.36.2` **есть** в offline-кеше `CARGO_HOME` ⇒ CP-7 Table-лента resolves без network;
  `make chown` → `chown: changing ownership of '/work': Operation not permitted` (CP-15 OPEN на тот момент).

### Evidence log — restore прав + git-reconciliation (2026-10-15 evening, measured)

- Ownership/writer access восстановлены operator-side (rootless idmap снова container-0 ↔ host 1003):
  `work/*` = `drwxrwxr-x opencode opencode` (`1003:1003`); host-side write probe в `work/aida64/out`
  (`touch` + `ls -l` + `rm`) = OK; mapping probe (`docker run --rm -v <repo>/work:/work busybox:1.37`
  создаёт `/work/_map-probe` как container-uid 0) ⇒ host видит файл как `1003:1003 644` и может его
  удалить. `make chown` больше не обязателен для host-видимости; `sudo -n` по-прежнему
  «a password is required» (no root).
- Git writer-access восстановлен: `.git/COMMIT_EDITMSG` (070), `.git/HEAD`, `.git/index` = owner
  `opencode` ⇒ `touch .git/_perm-probe` OK. Reconciliation chain: `git ls-remote --tags origin` =
  `v0.1`→`569ec8c`, `v0.2`→`ee5a058`, `v0.3`→`e50cfb6`, `v0.4`→`0df65b6`; `git fetch --tags` ⇒
  local tags `v0.1…v0.4`; local HEAD был `569ec8c` + staged-stale `AGENTS.md` blob ⇒
  `git reset --hard refs/remotes/origin/master` ⇒ HEAD = `0df65b6`, `git status --porcelain` empty,
  `git status -sb` = synced с `origin/master`; `git -C rea status --porcelain` empty.
- Release-chain post-`0df65b6`: Actions run `38781281003` `completed success` (`metadata`/`native`/
  `checks`/`publish` = success, publish updated 2026-10-15T17:57:01Z); release list = `v0.4` @
  `0df65b6` (published 17:57:01Z), `v0.3` @ `e50cfb6`, `v0.2` @ `ee5a058`, `v0.1` @ `569ec8c` —
  **Assets = 1** каждый ⇒ version-churn evidence для batch-policy (AGENTS rule 12, CP-14).
- Capability-map замеры (CP-4 gates): `uname` = `Linux nc-lab 6.12.86+deb13-amd64` (KVM guest:
  `/sys/class/dmi/id/product_name` = `KVM Server`, `bios_vendor` = `netcup`, `chassis_type` = 1;
  dmi-файлы `-r--r--r-- root`, values читаются unprivileged); PCI unprivileged OK (10 devices,
  `config` первые байты = `86 80 37 12` = Intel `0x8086:0x1237` host bridge); `/sys/class/hwmon` =
  **0** entries сегодня, `/sys/class/thermal` = 0 zones; `/dev/cpu` absent (`lsmod` даёт только
  `intel_rapl_msr` + `intel_rapl_common`, module `msr` не loaded); `/dev/port` = `crw-r----- root kmem`
  ⇒ MSR/physical port I/O require `modprobe msr` + perms (root-only) ⇒ MSR/sensor/port-ленты
  executables только на privileged или physical host.
- Следствие для ladder'а (обновлено этим же commit'ом): **CP-15 → DONE**, CP-3 write-blocker снят,
  CP-4 gate-числа/допуски переписаны (PCI/DMI/CPUID + fixture-лента here; MSR/sensors/ports — не here);
  CP-5/CP-9/CP-12/CP-13 остаются user-input gates (captures/display/baseline/licensing-решение).
