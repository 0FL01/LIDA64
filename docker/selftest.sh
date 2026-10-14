#!/bin/sh
# nutcracker RE toolkit - image self-proof.
#
# Runs in two modes:
#   core (default, at the end of the image build with SELFTEST_OUT=/tmp/selftest):
#     pinned runtime versions, REA prerequisites, ghidra provider availability,
#     scoped doctor, one real Ghidra analysis + exact-result snapshot reuse.
#   full (make selftest, SELFTEST_OUT=/work/selftest):
#     everything from core plus a Windows-PE + UPX contract proven on a PE that
#     the Ghidra archive already ships (no download, no host install).
#
# Assertions use the REAL shapes measured from rea 6.4.0 output:
#   doctor scope_checks items -> name / ok / classification / detail
#   providers candidates      -> .analysis_provider_candidates[].provider.id / .availability.status
#   analyze normalized_result -> document / segments / segment_count / procedure_count / string_count
#   analyze subject           -> name / digest.sha256 / format / architecture / local_path
#   function dossier          -> procedure / pseudocode / assembly / callers / callees / ...
set -eu

MODE="${SELFTEST_MODE:-core}"
OUT="${SELFTEST_OUT:-/work/selftest}"
ELF_TARGET="${SELFTEST_ELF_TARGET:-/bin/cat}"
PE_SEED="${SELFTEST_PE_SEED:-/opt/ghidra/docs/GhidraClass/ExerciseFiles/WinhelloCPP/WinHelloCPP.exe}"
mkdir -p "$OUT"

say() { printf '%s\n' "$*"; }
fail() { say "FAIL: $*"; exit 1; }
ms() { date +%s%3N; }
require() { # require <jq-program> <file> <label>
  jq -e "$1" "$2" >/dev/null || fail "$3 (jq probe: $1)"
}

say "== mode: ${MODE} =="
say "== runtime versions =="
node --version
npm --version
java -version 2>&1 | head -1
javac -version 2>&1
rea --help 2>&1 | head -3

say "== REA prerequisites present in the image (REA never installs these; the image does) =="
ps --version | head -1
upx --version | head -1
jq --version
file --version | head -1
objdump --version | head -1
rg --version | head -1
test -n "${GHIDRA_INSTALL_DIR:-}" || fail "GHIDRA_INSTALL_DIR missing"
test -x "$GHIDRA_INSTALL_DIR/support/analyzeHeadless" || fail "support/analyzeHeadless missing or not executable"
test -x "$GHIDRA_INSTALL_DIR/Ghidra/Features/Decompiler/os/linux_x86_64/decompile" || fail "linux_x86_64 native decompiler missing"
say "Ghidra install identity (REA validates the declared Java range itself):"
sed -n '/^application\./p' "$GHIDRA_INSTALL_DIR/Ghidra/application.properties"

say "== providers (ghidra must be available) =="
rea providers --json > "$OUT/providers.json"
require '.analysis_provider_candidates | type == "array" and length > 0' "$OUT/providers.json" "providers.json has analysis_provider_candidates"
jq -r '.analysis_provider_candidates[] | [.provider.id, .provider.version // "-", .availability.status, .availability.code // "-"] | @tsv' "$OUT/providers.json"
jq -e '.analysis_provider_candidates[] | select(.provider.id == "ghidra") | .availability.status == "available"' "$OUT/providers.json" >/dev/null || fail "ghidra provider not available"

say "== scoped doctor (provider ghidra) =="
rea doctor --provider ghidra --json > "$OUT/doctor-ghidra.json"
require '.healthy == true' "$OUT/doctor-ghidra.json" "doctor overall healthy"
require '.scope.mode == "explicit" and (.scope.providers | index("ghidra")) != null' "$OUT/doctor-ghidra.json" "doctor scope is explicit ghidra"
jq -r '.scope_checks[] | [.name, (.ok | tostring), .classification // "-"] | @tsv' "$OUT/doctor-ghidra.json"
require 'all(.scope_checks[]; .ok == true)' "$OUT/doctor-ghidra.json" "every ghidra scope check ok"
say "informational checks (capabilities deliberately outside this core image): count=$(jq -r '.informational_checks | length' "$OUT/doctor-ghidra.json")"

say "== cold analysis of ${ELF_TARGET} through the ghidra provider =="
# fresh ELF-phase artifacts so cold-vs-warm is deterministic on every rerun
rm -f "$OUT/elf-snapshot.json" "$OUT/analyze-elf-cold.json" "$OUT/analyze-elf-warm.json"
C0=$(ms)
rea analyze "$ELF_TARGET" --provider ghidra --snapshot "$OUT/elf-snapshot.json" --json > "$OUT/analyze-elf-cold.json"
C1=$(ms)
say "cold elapsed: $((C1 - C0)) ms"
require '.subject.format == "elf"' "$OUT/analyze-elf-cold.json" "subject format is elf"
require '.subject.architecture == "x86_64"' "$OUT/analyze-elf-cold.json" "subject architecture is x86_64"
require '.subject.digest.sha256 | type == "string" and length == 64' "$OUT/analyze-elf-cold.json" "subject digest sha256 present"
require '.normalized_result | type == "object" and length > 0' "$OUT/analyze-elf-cold.json" "normalized_result non-empty"
say "normalized keys: $(jq -c '.normalized_result | keys' "$OUT/analyze-elf-cold.json")"

say "== warm analysis (exact CLI snapshot reuse, read before the provider starts) =="
test -s "$OUT/elf-snapshot.json" || fail "snapshot file was not written"
W0=$(ms)
rea analyze "$ELF_TARGET" --provider ghidra --snapshot "$OUT/elf-snapshot.json" --json > "$OUT/analyze-elf-warm.json"
W1=$(ms)
say "warm elapsed: $((W1 - W0)) ms (cold: $((C1 - C0)) ms)"
test $((W1 - W0)) -lt $((C1 - C0)) || fail "warm run was not faster than the cold run"
say "warm token probe: $(rea analyze "$ELF_TARGET" --provider ghidra --snapshot "$OUT/elf-snapshot.json" --json --token-count)"

if [ "$MODE" != "full" ]; then
  say "SELFTEST CORE PASSED (artifacts: $OUT)"
  exit 0
fi

say "== Windows-PE proxy that the Ghidra archive already ships (no download) =="
# fresh PE-phase artifacts (idempotent reruns; stale outputs from earlier probes
# would make upx exit with FileAlreadyExistsException)
rm -f "$OUT/winpe.exe" "$OUT/winpe.upx" "$OUT/winpe.unpacked.exe" \
      "$OUT/packed-snapshot.json" "$OUT/unpacked-snapshot.json" \
      "$OUT/analyze-packed.json" "$OUT/analyze-unpacked.json" \
      "$OUT/function-entry.json" "$OUT/function-entry-warm.json" \
      "$OUT/decompile-entry.json" "$OUT/pe-resources.json" \
      "$OUT/managed-artifact.json" "$OUT/read-bytes.json" \
      "$OUT/bundle-from-record.json" "$OUT/analysis-record-boundary.txt" \
      "$OUT/view-native-pseudocode.json" "$OUT/native-view-pseudocode.json" \
      "$OUT/view-native-summary.json" "$OUT/view-jsapp.json" \
      "$OUT/summary-jsapp.json" "$OUT/analyze-jsapp.json"
test -f "$PE_SEED" || fail "PE proxy seed missing: $PE_SEED"
cp -f "$PE_SEED" "$OUT/winpe.exe"
say "seed: $(file -b "$OUT/winpe.exe")"

say "== UPX round-trip (unpack recipe: source bytes untouched) =="
upx -9 -q -o "$OUT/winpe.upx" "$OUT/winpe.exe" || fail "upx -9 could not pack the proxy PE"
say "packed: $(file -b "$OUT/winpe.upx")"
sha256sum "$OUT/winpe.exe" "$OUT/winpe.upx"
upx -d -q -o "$OUT/winpe.unpacked.exe" "$OUT/winpe.upx" || fail "upx -d could not unpack"
say "unpacked: $(file -b "$OUT/winpe.unpacked.exe")"
sha256sum "$OUT/winpe.exe" "$OUT/winpe.upx" "$OUT/winpe.unpacked.exe"
say "note: upx 4.2.4 round-trip is NOT byte-identical for an MSVC-linked PE; compare analysis content, not digests."

say "== packed vs unpacked analysis contrast (the AIDA64 phenomenon) =="
rea analyze "$OUT/winpe.upx" --provider ghidra --snapshot "$OUT/packed-snapshot.json" --json > "$OUT/analyze-packed.json"
rea analyze "$OUT/winpe.unpacked.exe" --provider ghidra --snapshot "$OUT/unpacked-snapshot.json" --json > "$OUT/analyze-unpacked.json"
require '.subject.format == "pe"' "$OUT/analyze-packed.json" "packed subject recognized as PE"
require '.subject.format == "pe"' "$OUT/analyze-unpacked.json" "unpacked subject recognized as PE"
PACKED_PROC=$(jq -r '.normalized_result.procedure_count' "$OUT/analyze-packed.json")
UNPACKED_PROC=$(jq -r '.normalized_result.procedure_count' "$OUT/analyze-unpacked.json")
say "procedure_count packed=$PACKED_PROC unpacked=$UNPACKED_PROC"
say "segments packed head:   $(head -c 400 "$OUT/analyze-packed.json")"
say "segments unpacked head: $(head -c 400 "$OUT/analyze-unpacked.json")"
rg -q 'UPX0' "$OUT/analyze-packed.json" || fail "packed analysis did not report the UPX0 section"
rg -q '\.text' "$OUT/analyze-unpacked.json" || fail "unpacked analysis did not report a .text section"
test "$PACKED_PROC" -lt 50 || fail "packed PE exposed more than 49 procedures ($PACKED_PROC); UPX stub assumption violated"
test "$UNPACKED_PROC" -gt 200 || fail "unpacked PE exposed <=200 procedures ($UNPACKED_PROC); unpack did not restore code"

say "== function dossier (cached kind of query: prefer this for repeated reads) =="
rea function "$OUT/winpe.unpacked.exe" entry --provider ghidra --snapshot "$OUT/unpacked-snapshot.json" --json > "$OUT/function-entry.json"
require '.normalized_result.procedure | type == "object" and length > 0' "$OUT/function-entry.json" "entry procedure dossier"
require '.normalized_result.pseudocode | type == "string" and length > 0' "$OUT/function-entry.json" "entry pseudocode present"
ADDR=$(jq -r '.normalized_result.procedure.address' "$OUT/function-entry.json")
say "entry address: $ADDR; callees: $(jq -c '[.normalized_result.callees[]?.name] // []' "$OUT/function-entry.json")"
F0=$(ms)
rea function "$OUT/winpe.unpacked.exe" entry --provider ghidra --snapshot "$OUT/unpacked-snapshot.json" --json > "$OUT/function-entry-warm.json"
F1=$(ms)
say "function warm repeat: $((F1 - F0)) ms (cold dossier above)"
test $((F1 - F0)) -lt 10000 || fail "function warm repeat took $((F1 - F0)) ms; snapshot replay should be under 10 s"

say "== decompile (this operation class replays cold: provider restarts per distinct query) =="
test -n "$ADDR" && test "$ADDR" != "null" || fail "could not extract entry address"
D0=$(ms)
rea decompile "$OUT/winpe.unpacked.exe" "$ADDR" --provider ghidra --json > "$OUT/decompile-entry.json"
D1=$(ms)
say "decompile elapsed: $((D1 - D0)) ms"
test -s "$OUT/decompile-entry.json" || fail "decompile produced no output"

say "== fast built-ins that need no engine =="
rea inspect-pe-resources "$OUT/winpe.unpacked.exe" --json > "$OUT/pe-resources.json"
test -s "$OUT/pe-resources.json" || fail "pe-resources empty"
say "pe-resources bytes: $(wc -c < "$OUT/pe-resources.json")"
rea inspect-managed-artifact "$OUT/winpe.unpacked.exe" --json > "$OUT/managed-artifact.json"
test -s "$OUT/managed-artifact.json" || fail "managed-artifact empty"

say "== snapshot-flag boundary (read-bytes takes no --snapshot) =="
rea read-bytes "$OUT/winpe.unpacked.exe" "$ADDR" --length 64 --json > "$OUT/read-bytes.json" || fail "read-bytes failed"
test -s "$OUT/read-bytes.json" || fail "read-bytes empty"
if rea read-bytes "$OUT/winpe.unpacked.exe" "$ADDR" --snapshot "$OUT/unpacked-snapshot.json" --json >/dev/null 2>&1; then
  fail "read-bytes accepted --snapshot although the CLI does not support it"
fi
say "read-bytes + --snapshot correctly rejected by the CLI flag surface"

say "== Evidence record vs Evidence bundle boundary (documented, asserted) =="
if rea evidence-export "$OUT/analyze-unpacked.json" "$OUT/bundle-from-record.json" --json > "$OUT/analysis-record-boundary.txt" 2>/dev/null; then
  fail "evidence-export accepted a single CLI Evidence record as if it were a bundle"
fi
rg -qi "Evidence bundle" "$OUT/analysis-record-boundary.txt" || fail "boundary message not recorded"
say "CLI --json result is ONE Evidence record: evidence-export fails closed (measured). Recorded to $OUT/analysis-record-boundary.txt"

say "== paging native analysis: analysis-view takes analyze_function, not binary_overview =="
jq -c '{source:{kind:"inline",evidence:.},view:{kind:"native",facet:"pseudocode"}}' "$OUT/function-entry.json" > "$OUT/view-native-pseudocode.json"
rea inspect-analysis-view "$OUT/view-native-pseudocode.json" --json > "$OUT/native-view-pseudocode.json" || fail "native facet view (pseudocode) failed"
test -s "$OUT/native-view-pseudocode.json" || fail "native facet view empty"
say "native facet view bytes: $(wc -c < "$OUT/native-view-pseudocode.json") (vs full dossier $(wc -c < "$OUT/function-entry.json"))"
jq -c '{source:{kind:"inline",evidence:.},view:{kind:"summary"}}' "$OUT/analyze-unpacked.json" > "$OUT/view-native-summary.json"
if rea inspect-analysis-view "$OUT/view-native-summary.json" --json > /dev/null 2>&1; then
  fail "summary view accepted a binary_overview record"
fi
say "summary kind rejected for parent operation binary_overview (measured: unsupported_target; supported parents are inspect_binary_layout, analyze_javascript_application, analyze_function)"

say "== engine-free record + summary paging (javascript workflow, no Ghidra needed) =="
JS="$OUT/jsapp"
rm -rf "$JS"
mkdir -p "$JS"
printf 'export function checksum(text) {\n  let h = 2166136261;\n  for (const c of String(text)) { h ^= c.charCodeAt(0); h = Math.imul(h, 16777619); }\n  return (h >>> 0).toString(16);\n}\nexport function parseFlag(args, flag) {\n  const p = "--" + flag + "=";\n  for (const a of args) if (a.startsWith(p)) return a.slice(p.length);\n  return null;\n}\n' > "$JS/index.js"
printf '{"name":"selftest-jsapp","version":"1.0.0","type":"module","main":"index.js"}\n' > "$JS/package.json"
rea analyze-javascript-application "$JS" --json > "$OUT/analyze-jsapp.json" || fail "analyze-javascript-application failed"
require '.normalized_result | type == "object" and length > 0' "$OUT/analyze-jsapp.json" "jsapp normalized_result non-empty"
jq -c '{source:{kind:"inline",evidence:.},view:{kind:"summary"}}' "$OUT/analyze-jsapp.json" > "$OUT/view-jsapp.json"
rea inspect-analysis-view "$OUT/view-jsapp.json" --json > "$OUT/summary-jsapp.json" || fail "summary paging of a jsapp record failed"
test -s "$OUT/summary-jsapp.json" || fail "summary-jsapp empty"
say "summary view bytes: $(wc -c < "$OUT/summary-jsapp.json") (vs full record $(wc -c < "$OUT/analyze-jsapp.json"))"

say "SELFTEST FULL PASSED (artifacts: $OUT)"
say "note: outputs written by uid 0 in the container; run 'make chown' to give them back to your host uid."
