# Recipe 02 — snapshots, Evidence, token economy

Модель стоимости (measured в этом образе, rea 6.4.0 + Ghidra 12.1.4): CLI-инвокация = отдельный процесс;
каждый **distinct** query через Ghidra = cold provider-сессия (**~18–31 s**). Exact-result snapshot replay
read'ся **до** старта provider'а → warm **~0.19–2.2 s** (measured warm `durationMs` 169–339 ms,
build-gate warm 1843–1905 ms vs cold 16896–18562 ms). Engine-free operations вообще не трогают engine
(**~24–90 ms**).

## 1. Что replay'ится, а что всегда cold (measured table)

| Warm replay с `--snapshot` | Всегда provider-cold | Engine-free (snapshot не нужен) |
| --- | --- | --- |
| `analyze` (overview) | `decompile` | `inspect-pe-resources` (~24 ms) |
| `search` | `instructions` | `inspect-managed-artifact` (~88 ms) |
| `function` (dossier) | `xrefs` | `analyze-javascript-application` (~0.55 s) |
| `trace` (reuses cached search observation) | `inspect-native-load-image` | `inspect-analysis-view` (~48 ms) |
| — | `read-bytes`, `address-to-file-offset` — **не принимают `--snapshot`** (`Unknown flag: --snapshot`, measured) | `evidence-import` boundary checks |

Source basis (не over-explain): `rea/src/application/binary/AnalysisSnapshotCache.ts` (eligibility:
exact operation+parameters+provider+settings, `cachePolicy`, `mutatesArtifact`, cursor-default ops),
`rea/src/ghidra/GhidraProviderCapabilities.ts` (`cachePolicy:"snapshot"` для read-ops, кроме mutating +
`list_documents`), `rea/docs/cli.md:177` (CLI cache hit read before provider start) и
`rea/docs/cli.md:185-189` (only eligible exact queries replay; new/live queries всё ещё могут require provider startup).
Practical guidance: dossier (`make function`) как primary repeated-read форма; snapshot для
`analyze`/`search`/`trace`; точечные `decompile`/`xrefs` планировать как cold (~27 s).

## 2. Snapshot layout (portable)

```
work/<target>/
  <name>.exe                      # target (gitignored)
  <name>.unpacked.exe             # derived by `make unpack`
  analysis/<snapshot>.json        # REA snapshot (exact-result cache), owner-only writes
  out/<cmd>-<target>.json         # Evidence records (--json output)
```

Примеры (measured, real files): `work/unpackdemo/analysis/aida64-unpacked.json` +
`work/unpackdemo/out/analyze-aida64.unpacked.exe.json`; warm run reuse'ит snapshot по тому же имени:
`TIMEOUT=900000 SNAPSHOT=aida64-unpacked.json make analyze` → exit 0, sub-second `durationMs`.
Snapshot cache key покрывает result-affecting inputs (bytes/operation/parameters/provider/settings), а не
path-impurity: после `upx -d` digest меняется → нужен отдельный snapshot для unpacked target.

Правила semantics (REA docs/cli.md): metadata-write invalidates active session cache; close/reopen before
importing/saving another snapshot; cursor-dependent и mutating ops excluded; snapshot replay не означает, что
"новая" функция уже проанализирована — cold-плате plausible для fresh analysis.

## 3. Evidence boundary: record vs bundle (measured, fail-closed)

CLI `--json` результат = **один Evidence record** (keys: `evidence_id, subject, provider, analysis_profile,
predicate_type, operation, parameters, raw_result, normalized_result, confidence, authority, environment`,
иногда `evidence_links/limitations/locations`), НЕ bundle с `records`/`manifest` arrays. Поэтому:

```bash
rea evidence-export /work/aida64/out/analyze-….json /work/aida64/out/bundle.json   # -> fail closed
# code: evidence_integrity_mismatch | "Expected an Evidence bundle with records and manifest arrays,
#  but this JSON is one Evidence record (ev_...)"  (measured; recorded in work/selftest/analysis-record-boundary.txt)
rea compare <left> <right>                                                          # bundles only
```

Bundles produces MCP tool `export_evidence_bundle` (или `rea evidence-export` уже от bundle-file).
CLI-версия MCP-сервера (`rea mcp`) в этом контракте не используется (CLI-only topology).

## 4. Paging вместо big reads (agent-token discipline)

`rea inspect-analysis-view` требует **positional** `inputJson` (stdin не читает — measured: piping даёт
CLI-layer `VALIDATION_ERROR` с `fieldErrors`), и принимает envelope только с parent operation
`analyze_function` / `analyze_javascript_application` / `inspect_binary_layout`:

```bash
TARGET=aida64 make analysis-view FILE=function-aida64.unpacked.exe-entry.json FACET=pseudocode
# envelope, который Makefile строит сам: {"source":{"kind":"inline","evidence":<record>},"view":{"kind":"native","facet":"pseudocode"}}
```

Measured shapes: dossier 20879 B → native page 7993 B (`item.unit = "utf16-code-units"`,
`procedure_address "0x401e46"`, `parent_operation` echo); JS record 135463 B → summary page 9763 B.
Inverted case measured: `kind:"summary"` на record от `rea analyze` → `unsupported_target` с remediation.
CLI-слой vs domain-слой error shapes различаются: CLI arg errors = uppercase `VALIDATION_ERROR` + `fieldErrors`;
workflow errors = lowercase codes (`unsupported_target`, `evidence_integrity_mismatch`) + `category`/`remediation`.

## 5. Token economy без engine-restart

```bash
TARGET=aida64 SNAPSHOT=aida64.json make analyze TOKENS=2000          # --token-limit (offset: --token-offset)
TARGET=aida64 SNAPSHOT=aida64.json make analyze EXTRA='--token-count'  # measured: warm overview = 1965 tokens
TARGET=aida64 SNAPSHOT=aida64.json make analyze FILTER='normalized_result.procedure_count'  # --filter-output
```

Measured timings для этих probes: ~0.15–0.2 s (no provider). `--format toon|yaml|md|jsonl`, `--full-output`,
`--llms`, `--schema` тоже доступны (`--schema --json` у `inspect-analysis-view` даёт только arg-schema).

## 6. Housekeeping

```bash
make chown     # container writes как uid 0 → верни файлы host-пользователю (rootless note: recipe 00 §7)
make clean     # rm derived outputs work/aida64/{out,analysis} work/selftest (targets и образ не трогает)
```

Не kommit'я: `work/*` (targets, Evidence, snapshots), binaries of AIDA64 (commercial, license-hold).
Commit'я: `docker/*`, `Makefile`, `recipes/*`, `README.md`, `PLAN.md`, `.gitignore`, `work/.gitkeep`.
