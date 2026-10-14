# Recipe 01 — AIDA64 static-путь (Linux headless, без GUI и без Windows-execution)

Prerequisite: recipe 00 выполнен (`make build`, `make selftest` green). Target: Windows PE x64/x86,
положенный пользователем (license-hold) как `work/aida64/aida64.exe`.

Факт про packing: AIDA64 uses standard UPX packing; `upx -d aida64.exe` разворачивает его в normal exe,
после чего image анализируется Ghidra/IDA (teardown AIDA64 8.25.8200). REA не имеет own UPX-unpacker'а →
в образ включён `upx` (4.2.4). Без разупаковки анализ видит только stub (measured proxy: procedures 7,
segments `Headers/UPX0/UPX1/.rsrc/tdb`), после разупаковки — реальный код (procedures 388, segments
`Headers/.text/.rdata/.data/.rsrc/tdb`).

## 1. Unpack (source bytes untouched)

```bash
make unpack                                  # SRC=aida64.exe по default; idempotent (rm -f только derived output)
# or: TARGET=aida64 SRC=aida64.exe make unpack
```

Writes `work/aida64/aida64.unpacked.exe` и печатает digests источника и unpacked + `file` output.
Fail-closed свойства `upx`: refuse overwrite (поэтому `rm -f` derived output внутри) и
`NotPackedException` для уже unpacked input. Round-trip UPX 4.2.4 **не byte-identical** для MSVC-linked PE
(110592 → 62464 → 110592 B) → сравнивай содержимое анализа, а не digests. Важно: `SRC` должен заканчиваться
на `.exe`, иначе `$(UNPACKED)` = `$(subst .exe,…)` совпадёт с source.

## 2. Overview (cold) + snapshot

```bash
TIMEOUT=900000 SNAPSHOT=aida64.json make analyze
# -> work/aida64/out/analyze-aida64.unpacked.exe.json  +  work/aida64/analysis/aida64.json
```

Measured (proxy seed): `{procedures:388, segments:6, strings:256, format:"pe", architecture:"x86"}`.
Ghidra JVM startup+import+auto-analysis = **~18–31 s** per cold query здесь; default
`REA_GHIDRA_STARTUP_TIMEOUT_MS=330000` may быть недостаточен для больших images → `TIMEOUT=900000`
(или выше; max 2147483647 ms). Projection without engine:

```bash
jq -c '{procedures: .normalized_result.procedure_count, segments: .normalized_result.segment_count,
       strings: .normalized_result.string_count, fmt: .subject.format, arch: .subject.architecture,
       digest: .subject.digest.sha256[0:12], path: .subject.local_path}' \
  work/aida64/out/analyze-aida64.unpacked.exe.json
```

`locations` и `raw_result` у `analyze` empty (measured) → counts live в `normalized_result`.

## 3. Engine-free triage (fast, ms-level)

```bash
make pe-resources        # ~24 ms, ~20 KB   -> out/pe-resources-aida64.unpacked.exe.json
make managed-artifact    # ~88 ms, exit 0 даже для non-.NET PE   (отделяет PE/CLI assembly от native)
TARGET=aida64 BIN=aida64/aida64.unpacked.exe ADDR=0x401e46 make bytes LENGTH=64
TARGET=aida64 BIN=aida64/aida64.unpacked.exe ADDR=0x401e46 make offset
```

`inspect-native-load-image` (`make load-image`) даёт measurements, но для PE attestation статус
`unsupported` (REA semantics, documented) → не интерпретируй его как verification of load correctness.

## 4. Workhorse queries (cached kinds)

```bash
make search QUERY='license' SNAPSHOT=aida64.json
make trace  QUERY='license' SNAPSHOT=aida64.json
make function NAME=entry    SNAPSHOT=aida64.json        # dossier: pseudocode+assembly+callers+callees+refs+api
make function NAME=0x401e46 SNAPSHOT=aida64.json        # адрес тоже принимается; 'entry' = entry function
```

Dossier normalized keys (measured): `procedure, pseudocode, assembly, comments, callers, callees,
incoming_references, outgoing_references, referenced_strings, referenced_names, basic_blocks, native_api,
native_value_flow, limitations`. Это **самая выгодная** read-форма: dossier содержит псевдокод И assembly И refs
одновременно и replay'ится warm (~0.2–2.2 s). `decompile`/`instructions`/`xrefs` — provider-cold (~27–31 s)
даже с snapshot → используй их точечно.

## 5. Paging без engine (agent/token-friendly)

```bash
TARGET=aida64 make analysis-view FILE=function-aida64.unpacked.exe-entry.json FACET=pseudocode
TARGET=aida64 make -s analysis-view FILE=analyze-jsapp.json VIEW_KIND=summary FACET=   # example для JS-records
```

Restriction (measured, fail-closed): `view:{kind:"native",facet:…}`/`kind:"summary"` принимает только records
с parent operation `analyze_function`, `analyze_javascript_application` или `inspect_binary_layout`; record от
`rea analyze` (parent `binary_overview`) reject'ится → use dossier для paging. Facets: `pseudocode, procedure,
assembly, basic_blocks, comments, callers, callees, references, unresolved-calls, referenced-strings,
referenced-names, native-api, high-pcode`; page units = UTF-16 code units (`item.unit` measured), `item.text`
= страница. Token economy без engine-restart: `TOKENS=2000` (`--token-limit`), `EXTRA="--token-count"`.

## 6. Tuning knobs (exact names/formats)

| Knob | Как | Когда |
| ---- | --- | ----- |
| `TIMEOUT=900000` | `-e REA_GHIDRA_STARTUP_TIMEOUT_MS` | большие PE / медленные hosts |
| `MAXMEM=1G` | `-e GHIDRA_HEADLESS_MAXMEM` | tight RAM (compose `mem_limit` тоже reduce: `MEM=6g`) |
| `DISABLED_ANALYZERS=WindowsResourceReference` | exact Ghidra analyzer name | PE-resource analyzer stall |
| `SEED_FILE=/work/aida64/seeds.tsv` | `0xADDR<TAB>function|code|label[<TAB>name]`, `#` comments | computed jumps / code в data |
| `LANGUAGE_ID=x86:LE:32:default` (+`COMPILER_SPEC_ID=borlandcpp`) | overrides для import | Borland/32-bit cspec; spec требует language |

All optional knobs подаются `-e` **только непустыми** (Makefile делает это сам): пустое значение
`REA_GHIDRA_LANGUAGE_ID=` reject'ится REA zod-схемой (measured: direct `-e REA_GHIDRA_LANGUAGE_ID=` →
`rea` exit 1; `make providers LANGUAGE_ID=` → exit 0, потому что флаг omission'ится).

## 7. Interpretation limits (must сохранять в notes)

- Linux-host Ghidra анализирует Windows PE статически: image не исполняется; `rea` не умеет
  Windows-host process capture → динамика AIDA64 вне этого контракта.
- Pseudocode = provider-specific derivation, never original source; unresolved computed calls remain unknown.
- `annotate-native-function`/`set-address-names` доступны на Linux/macOS host, но это metadata-edits
  ephemeral DB: they discarded on close → фиксируй имена/notes в own files/snapshot, не полагаясь на DB.
- Evidence outputs = container paths (`/work/aida64/…`) → bundles/snapshots переносимы между машинами.
- После container-писей: `make chown` (rootless note в recipe 00).

## 8. Proof trail (already measured in this image)

`work/unpackdemo/` содержит полный AIDA64-shaped прогон на UPX-PE с именем `aida64.exe`
(seed = `WinHelloCPP.exe` из Ghidra-архива, packed digest `6ae26365c5cb…`):
unpack → `aida64.unpacked.exe` (digest `125628c5d990…`); packed analyze `{7,5,12}` vs unpacked analyze
`{388,6,256}`; snapshots `work/unpackdemo/analysis/aida64-{packed,unpacked}.json`; warm replay measured
`durationMs` 169–339 ms; ownership восстановлен. Повтори тот же путь с реальным `work/aida64/aida64.exe`.
