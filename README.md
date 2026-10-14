# nutcracker — Docker-only RE toolkit (первый target: AIDA64 Extreme)

Минимальная среда реверс-инжиниринга: на хосте нужен **только Docker + Compose v2** (+ GNU make для
рецептов). Node, JDK, Ghidra, `rea`, `upx`, `jq`, `rg`, `file`, `binutils`, `procps` — внутри образа.
`./rea` — REA checkout v6.4.0 как reference-код; он **не изменялся** этим work.

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
make chown                                        # вернуть файлы host-пользователю
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
  только статический анализ; GUI/extended-tools/MCP-агент-тотология вне scope (решения пользователя).

## Licenses (short)

REA-authored code MIT; Ghidra Apache-2.0; Node MIT (+ISC-зависимости); Temurin JDK GPL-2.0 + CE;
UPX GPL-2.0 (+ artistic/absolute-no-warranty per upstream); AIDA64 — commercial, license-hold
(бинарники в `work/` gitignored'ены). Юридические решения за пользователем.
