# Recipe 00 — хост, сборка, selftest, ownership

Host prerequisites: **Docker + Compose v2** и GNU make — ничего больше (no Node/Java/Ghidra/upx/jq на хосте).
x86_64 host (Ghidra native decompiler `linux_x86_64`; compose ставит `platform: linux/amd64`), ~5 GB free RAM,
≥10 GB free disk (image **2.73 GB** + ephemeral Ghidra DBs).

## 1. Host check (one-off)

```bash
docker --version && docker compose version && make --version
docker info --format '{{.Rootless}}'        # true здесь (measured) — влияет на ownership (шаг 4)
```

## 2. Bootstrap work dirs (host side)

```bash
mkdir -p work/aida64/out work/aida64/analysis
```

`work/` bind-монтируется в `/work`; REA и recipe-outputs пишут туда. `work/*` gitignore'ан (only `.gitkeep` tracked).

## 3. Build (digest-verified, fail-closed)

```bash
docker compose -f docker/compose.yaml build     # = make build
```

Gates внутри сборки (observed green):
- `sha256sum` совпадение для Node tarball / JDK tarball / Ghidra zip; mismatch → build exit ≠0.
- `Ghidra/application.properties` must contain `application.version=12.1.4` и Java-диапазон, admission'ный JDK 21
  (measured values: `application.java.min=21`, `application.java.max=` пустой).
- `support/analyzeHeadless` и `Ghidra/Features/Decompiler/os/linux_x86_64/decompile` executable (chmod +x defensively).
- build-time selftest (`core`-mode): versions (`node v24.18.0`, `openjdk 21.0.12.1`, javac 21, `rea@6.4.0` banner),
  prerequisites (`ps` procps-ng 4.0.4, `upx 4.2.4`, `jq-1.7`, `file-5.46`, objdump 2.44), providers-assertion
  (`ghidra available`; `hopper executable_missing`; `ida not_configured`), scoped doctor assertion
  (`.healthy == true` и `all(.scope_checks[].ok == true)`), затем cold/warm ELF (`/bin/cat`).
- Ghidra zip digest bootstrap: если `GHIDRA_SHA256=PENDING_VERIFY_AND_PIN` (первичный state), build печатает
  измеренный digest и exit 1 с инструкцией «запиши в ARG» (Ghidra не publishes checksum-артефакт).
  Текущий pinned digest: `ddac49f9…d2d4db` (verify-and-pin выполнен).

Ghidra mirror fallback (если GitHub-asset недоступен): SourceForge project `ghidra.mirror` path
`Ghidra_12.1.4_build/ghidra_12.1.4_PUBLIC_20260921.zip` (documented comment в Dockerfile); digest must совпасть.

## 4. Pin maintenance

```bash
make pin-check   # сравнивает ARG digests с upstream (Node SHASUMS256.txt, Adoptium .sha256.txt, Ghidra stream)
make pin         # + --update: переписывает ARG-строки digests в docker/Dockerfile
```

Measured green: `OK node/jdk/ghidra … All digest pins match upstream. Image builds are reproducible.` exit 0.
После любой правки pin: `make build && make selftest`.

## 5. Selftest (two modes)

```bash
make selftest-core   # build-gate subset: versions + prerequisites + providers + doctor + ELF cold/warm
make selftest        # + Windows-PE/UPX + function dossier + native-view paging + Evidence-boundary + engine-free JS
```

`make selftest` запускает **образную** копию `/usr/local/selftest.sh` (Dockerfile `COPY selftest.sh …`), поэтому
правки `docker/selftest.sh` требуют `make build` перед тем, как они подействуют — это была причина двух ранних
failure'ов. Script идемпотентен: он rm-ит свои artifacts перед cold/warm и PE-фазами (stale outputs иначе дают
`upx: FileAlreadyExistsException`).

Measured full-green run (`SELFTEST FULL PASSED (artifacts: /work/selftest)`):
- ELF: cold 18358–18562 ms → warm 1843–1905 ms; `--token-count` warm overview = **1965** tokens.
- PE seed = `WinHelloCPP.exe` из Ghidra-архива (no download): packed `procedure_count=7`, `segment_count=5`
  (UPX0/UPX1), unpacked `procedure_count=388`, `segment_count=6` (`.text`); entry `0x401e46`.
- function dossier warm replay 2220 ms (assertion `< 10000 ms`); decompile cold 31033 ms.
- `read-bytes` OK, `read-bytes --snapshot` correctly rejected; `evidence-export <record>` correctly rejected
  (message записан в `work/selftest/analysis-record-boundary.txt`).
- native facet view 7993 B vs dossier 20879 B; JS summary 9763 B vs record 135463 B.

Outputs host-visible: `work/selftest/…` (providers.json, doctor-ghidra.json, analyze-*.json, snapshots, views).

## 6. Capability readiness (как читать doctor правильно)

```bash
make doctor        # scoped --provider ghidra; prints healthy + scope_checks (name/ok/classification) + assertion
make providers     # candidates: ghidra available | hopper unavailable | ida unavailable
make capabilities  # {"count":18}
```

Scoped report'ы решают `healthy`; отсутствующие capabilities (`hopper`, `ida`, PTY/Frida/Chrome/pwntools/JADX/tshark)
попадают в `informational_checks` (measured count=4 в full аудите) → **не** failure этого контракта. `ida`
admission'ится через `REA_IDA_MCP_CONFIG` (absolute path), `hopper` — через `/opt/hopper/bin/Hopper` или
`HOPPER_LAUNCHER_PATH`; оба deliberately не установлены в core-образе.

## 7. Ownership (rootless Docker, measured)

Контейнер пишет как uid 0. В rootless-среде uid 0 внутри ↔ host uid владельца демона, а host-uid внутри
контейнера — это mapping на **другой** host uid (observed: host sees 297610 when container chowned to 1003).
Следствия и fix:

```bash
make chown        # chown -R 0:0 /work внутри -> host видит СВОЙ uid, host-side mkdir/rm снова работают
```

Ранние failure'ы (`Permission denied` на host-side mkdir, cascade exit 2 у analyze/unpack) были вызваны chown'ом
к хост-uid внутри контейнера. Если Docker daemon root-run — container writes land as root и нужна host-side
`sudo chown -R <you> work`. Run `make chown` после container-писей (или перед host-операциями с `work/`).

## 8. Dry-run без engine (fast smoke, ~<1 c per command)

```bash
make -n analyze TARGET=aida64                 # что будет выполнено (make expansion guard)
TARGET=selftest BIN=selftest/winpe.unpacked.exe ADDR=0x401e46 make -s bytes   # 5 keys: address/bytes_hex/complete/requested/returned
TARGET=selftest BIN=selftest/winpe.unpacked.exe ADDR=0x401e46 make -s offset  # 2 keys: address/file_offset
```

Multi-op batches: environment shell = zsh → используй `bash -c '…'` (failing globs/glob-no-match abort batches).
