# nutcracker — Docker-only RE toolkit (первый target: AIDA64 Extreme)

**Продуктовая цель: полный клон AIDA64 — native Linux** (Windows-платформа вне scope), с двумя
одинаково обязательными контурами parity: **visual/frontend** (пиксельная геометрия/палитра/
типографика UI AIDA64) и **feature** (тот же user-visible функционал на Linux-источниках).
Substrate цели: минимальная Docker-only RE-среда — на хосте для неё нужен **только Docker + Compose v2**
(+ GNU make для рецептов); Node, JDK, Ghidra, `rea`, `upx`, `jq`, `rg`, `file`, `binutils`, `procps` —
внутри образа. UI-красты (`ui/aida64-theme`, `ui/aida64-shell`) собираются host-side cargo'ом
(`CARGO_HOME=/home/opencode/.cargo`). `./rea` — REA checkout v6.4.0 как reference-код; он
**не изменялся** этим work. CI/release: `.github/workflows/native-release.yml`
(`metadata` → `checks` ∥ `native` → `publish`, Rust 1.96.1, `--locked`); push в `master`
⇒ GitHub release `v<run>` (measured: `v0.1`…`v0.4`, 1 asset each — version = run number, bump only by push).

Agent-facing контракт (правила, measured facts, layout, check ladder): **[`AGENTS.md`](AGENTS.md)**.
Contract со статусом и доказательствами: **[`PLAN.md`](PLAN.md)**.

## Quickstart

```bash
mkdir -p work/aida64/out work/aida64/analysis     # host-side bootstrap
docker compose -f docker/compose.yaml build       # image, digest-verified pins (runs core selftest)
make selftest                                     # full self-proof (ELF + Windows-PE/UPX + Evidence boundary)
# Положить AIDA64 build в work/aida64/ (zip или aida64.exe), затем:
make unpack SRC=aida64.exe                        # upx -d -> aida64.unpacked.exe (source bytes untouched)
TIMEOUT=1800000 MEM=6g SNAPSHOT=aida64-unpacked.json TARGET=aida64 BIN=AIDA64/aida64.unpacked.exe make analyze
make function NAME=entry SNAPSHOT=aida64-unpacked.json   # dossier: pseudocode+assembly+callers+callees+refs
make chown                                        # idmap-зависимо: measured EPERM в uid-blocked daemon-сессии, OK после restore ⇒ мерить stat -c %u:%g (PLAN §Ownership/idmap)
# UI-красты (host cargo, offline-кеш CARGO_HOME):
cd ui/aida64-theme && cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check   # 57 tests green
cd ui/aida64-theme && python3 tools/model_vs_capture.py   # golden-сверка → work/aida64/out/ui-golden-vs-captures.txt (30 PASS)
cd ui/aida64-shell && cargo build && cargo clippy --all-targets && cargo fmt --check   # compile-gate; window run = display backlog
```

## Recipes (подробно)

| Recipe | О чём |
| ------ | ----- |
| [`recipes/00-build-selftest.md`](recipes/00-build-selftest.md) | сборка, readiness (doctor/providers), Ghidra-digest bootstrap, pin maintenance, rootless-ownership, engine-free smokes |
| [`recipes/01-aida64-static.md`](recipes/01-aida64-static.md) | UPX-разупаковка → Ghidra overview → search/trace/function dossier → paging (native facets) → tuning knobs (seeds/analyzers/heap/timeout) |
| [`recipes/02-snapshots-evidence.md`](recipes/02-snapshots-evidence.md) | measured cost-модель (cold vs warm vs engine-free), snapshot rules, Evidence record-vs-bundle boundary, token-economy flags |

## Kраткий профиль

- Path convention: всё в bind-mounted `./work` → Evidence содержит контейнерные `/work/<target>/…` (bundle
  переносим между машинами). Портов у сервиса нет (CLI-only).
- Memory: `mem_limit: ${REA_MEM:-4GiB}`; Ghidra JVM `-Xmx` = `GHIDRA_HEADLESS_MAXMEM` (baked `2G`).
- Pins (Node 24.18.0, JDK 21.0.12.1+1, Ghidra 12.1.4, `rea-agents` 6.4.0) живут в `ARG`-блоке
  `docker/Dockerfile`; обновление: `make pin` → `make build` → `make selftest`.
- AIDA64 как Windows-приложение здесь не запускается (REA process-capture требует Linux/macOS host) —
  только статический анализ; **Windows как product-платформа вне scope** (клон = native Linux),
  GUI/extended-tools/MCP-агент-тотология вне scope (решения пользователя). Прогресс по цели фиксируется
  **только** checkpoint-гейтами и ledger'ом в [`PLAN.md`](PLAN.md) — «почти готово» без evidence не считается.
- Writer/ownership state **session-dependent** (rootless idmap + owner of `work/` и `.git`; measured
  оба состояния 2026-10-15): если host-uid не может писать в `work/` или `.git` — procedure в
  [`AGENTS.md`](AGENTS.md) rules 6/10; drift local-HEAD чистится `git fetch` +
  `git reset --hard refs/remotes/origin/master` (tags — `git fetch --tags`).

## Licenses (short)

REA-authored code MIT; Ghidra Apache-2.0; Node MIT (+ISC-зависимости); Temurin JDK GPL-2.0 + CE;
UPX GPL-2.0 (+ artistic/absolute-no-warranty per upstream); AIDA64 — commercial, license-hold
(бинарники в `work/` gitignored'ены). Юридические решения за пользователем.
