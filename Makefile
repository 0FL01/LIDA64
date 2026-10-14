# nutcracker RE toolkit - portable recipe entry points.
#
# Host requirement: GNU make + Docker Compose v2 only. jq/rea/upx/Ghidra/JDK/Node
# all live in the image; assertions run inside it too (no host toolchain).
#
# Analysis targets use container paths (/work/...) and write Evidence to
# work/<TARGET>/out|analysis on the host, so results are machine-portable.
#
# Measured in this image (rea 6.4.0 + Ghidra 12.1.4, small x86 PE):
#   cold Ghidra session per distinct query  ~26-28 s
#   warm exact-result snapshot replay       ~0.17-0.25 s   (analyze/search/function/trace)
#   decompile / instructions / xrefs / load-image keep restarting the provider (~27 s)
#   read-bytes / address-to-file-offset accept no --snapshot at all
#   built-ins with no engine at all         ~24-90 ms       (pe-resources, managed-artifact)

COMPOSE  ?= docker compose -f docker/compose.yaml
MEM      ?= 4294967296
TARGET   ?= aida64
SRC      ?= aida64.exe
UNPACKED = $(subst .exe,.unpacked.exe,$(SRC))
# BIN is relative to /work/<TARGET>/ (set TARGET for the directory, BIN for the file:
# TARGET=aida64 BIN=AIDA64/aida64.exe addresses /work/aida64/AIDA64/aida64.exe)
BIN      ?= $(UNPACKED)
SNAPSHOT ?=
TIMEOUT  ?=
MAXMEM   ?=
DISABLED_ANALYZERS ?=
LANGUAGE_ID        ?=
COMPILER_SPEC_ID   ?=
SEED_FILE          ?=
ADDR     ?=
NAME     ?=
QUERY    ?=
FILE     ?=
VIEW_KIND ?= native
FACET    ?= pseudocode
P_OFFSET ?=
P_LIMIT  ?=
LENGTH   ?= 256
TOKENS   ?=
FILTER   ?=
EXTRA    ?=

# Optional REA overrides are passed only when non-empty: REA's zod environment
# schema rejects empty values (rea/src/config/environment.ts), so `-e NAME=`
# would fail closed. `-e` must precede the service name in `compose run`.
GHIDRA_ENV = \
 $(if $(TIMEOUT),-e REA_GHIDRA_STARTUP_TIMEOUT_MS=$(TIMEOUT),) \
 $(if $(MAXMEM),-e GHIDRA_HEADLESS_MAXMEM=$(MAXMEM),) \
 $(if $(DISABLED_ANALYZERS),-e REA_GHIDRA_DISABLED_ANALYZERS='$(DISABLED_ANALYZERS)',) \
 $(if $(LANGUAGE_ID),-e REA_GHIDRA_LANGUAGE_ID='$(LANGUAGE_ID)',) \
 $(if $(COMPILER_SPEC_ID),-e REA_GHIDRA_COMPILER_SPEC_ID='$(COMPILER_SPEC_ID)',) \
 $(if $(SEED_FILE),-e REA_GHIDRA_SEED_FILE='$(SEED_FILE)',)
SNAP  = $(if $(SNAPSHOT),--snapshot /work/$(TARGET)/analysis/$(SNAPSHOT),)
TOK   = $(if $(TOKENS),--token-limit $(TOKENS),)
FILT  = $(if $(FILTER),--filter-output '$(FILTER)',)
FLAGS = $(TOK) $(FILT) $(EXTRA)
RUN   = REA_MEM=$(MEM) $(COMPOSE) run --rm -T $(GHIDRA_ENV) rea
OUTDIR = work/$(TARGET)/out
ADIR   = work/$(TARGET)/analysis
IN     = /work/$(TARGET)/$(BIN)
JQ     = --json

.PHONY: help build pin pin-check selftest selftest-core doctor providers capabilities \
        shell unpack analyze decompile function search trace xrefs instructions \
        bytes offset pe-resources load-image managed-artifact analysis-view evidence \
        workdirs check-args chown clean

help:
	@printf '%s\n' \
	 'make build                       # build image layers (digest-verified pins)' \
	 'make selftest                    # full self-proof: doctor + ELF + Windows-PE/UPX contract' \
	 'make selftest-core               # build-time gate only (no PE/UPX phase)' \
	 'make unpack SRC=aida64.exe       # upx -d -> <name>.unpacked.exe (source bytes untouched)' \
	 'make analyze [SNAPSHOT=f.json]   # overview -> $(OUTDIR)/analyze-<name>.json' \
	 'make function NAME=main          # dossier: pseudocode+assembly+callers+callees+refs (cached kind)' \
	 'make search QUERY=text | make trace QUERY=text' \
	 'make decompile|instructions|xrefs ADDR=0x401e46   # note: these replay cold (~27 s) per distinct query' \
	 'make bytes ADDR=0x401e46 LENGTH=64 | make offset ADDR=0x401e46   # engine-free, no --snapshot' \
	 'make pe-resources | managed-artifact   # engine-free built-ins (~24-90 ms)' \
	 'make load-image                  # measured load mappings (PE attestation reported unsupported)' \
	 'make analysis-view FILE=function-<name>.json [FACET=pseudocode]   # paged native view; FILE=analyze-jsapp.json VIEW_KIND=summary FACET=' \
	 'make doctor | providers | capabilities' \
	 'make evidence                    # bundle vs record boundary help' \
	 'make shell                       # interactive shell inside the environment' \
	 'make chown                       # give host files back to your uid' \
	 'make pin | pin-check             # verify/refresh digest pins' \
	 '' \
	 'Knobs: MEM=6g TIMEOUT=900000 MAXMEM=1G DISABLED_ANALYZERS=... LANGUAGE_ID=... COMPILER_SPEC_ID=... SEED_FILE=/work/...' \
	 'Token economy: TOKENS=2000 FILTER=normalized_result.procedure_count EXTRA="--token-count" (no engine restart needed)' \
	 'Example: TIMEOUT=900000 SNAPSHOT=aida64.json make analyze TARGET=aida64 BIN=AIDA64/aida64.unpacked.exe'
	 'Layout: a portable AIDA64 zip extracts under work/aida64/AIDA64/ -> BIN=AIDA64/aida64.unpacked.exe'

build:
	$(COMPOSE) build

pin:
	sh docker/pin.sh

pin-check:
	sh docker/pin.sh --check

# core mode is the build-time gate; full mode adds the Windows-PE + UPX contract
selftest: workdirs
	$(COMPOSE) run --rm -T -e SELFTEST_MODE=full rea /usr/local/selftest.sh

selftest-core: workdirs
	$(COMPOSE) run --rm -T -e SELFTEST_MODE=core rea /usr/local/selftest.sh

# rea/upx/jq run inside the image (no host toolchain); reports land in ./work/...
doctor: workdirs
	@$(RUN) sh -c 'rea doctor --provider ghidra --json | tee /work/$(TARGET)/analysis/doctor-ghidra.json >/dev/null; jq -r "{healthy: .healthy, environment_healthy: .environment_healthy, scope_mode: .scope.mode, providers: .scope.providers, informational_checks: (.informational_checks | length)}" /work/$(TARGET)/analysis/doctor-ghidra.json; printf "%s\n" "--- scope_checks (name / ok / classification) ---"; jq -r ".scope_checks[] | [.name, (.ok|tostring), (.classification // \"-\")] | @tsv" /work/$(TARGET)/analysis/doctor-ghidra.json; jq -e ".healthy == true and all(.scope_checks[]; .ok == true)" /work/$(TARGET)/analysis/doctor-ghidra.json >/dev/null'

providers: workdirs
	@$(RUN) sh -c 'rea providers --json | tee /work/$(TARGET)/analysis/providers.json >/dev/null; jq -c ".analysis_provider_candidates | length" /work/$(TARGET)/analysis/providers.json; jq -r ".analysis_provider_candidates[] | [.provider.id, (.provider.version // \"-\"), .availability.status, (.availability.code // \"-\")] | @tsv" /work/$(TARGET)/analysis/providers.json; jq -e ".analysis_provider_candidates[] | select(.provider.id==\"ghidra\") | .availability.status == \"available\"" /work/$(TARGET)/analysis/providers.json >/dev/null'

capabilities: workdirs
	@$(RUN) sh -c 'rea capabilities --json | tee /work/$(TARGET)/analysis/capabilities.json >/dev/null; jq -c ".capabilities | if type==\"array\" then {count: length} else {keys: keys} end" /work/$(TARGET)/analysis/capabilities.json'

shell:
	$(COMPOSE) run --rm -it rea bash

workdirs:
	@mkdir -p $(OUTDIR) $(ADIR)

check-args:
	@test -z "$(COMPILER_SPEC_ID)" -o -n "$(LANGUAGE_ID)" || \
	  { echo "COMPILER_SPEC_ID requires LANGUAGE_ID (REA validates them together)"; exit 1; }

# idempotent: only the derived output is removed inside the container (root);
# the source file is never touched, so its digest stays verifiable afterwards.
unpack: workdirs
	$(RUN) sh -c 'rm -f "/work/$(TARGET)/$(UNPACKED)"; upx -d "/work/$(TARGET)/$(SRC)" -o "/work/$(TARGET)/$(UNPACKED)"'
	@printf 'source bytes unchanged: '; sha256sum work/$(TARGET)/$(SRC)
	@printf 'unpacked bytes:       '; sha256sum work/$(TARGET)/$(UNPACKED)
	@file work/$(TARGET)/$(UNPACKED)

analyze: workdirs check-args
	$(RUN) rea analyze $(IN) --provider ghidra $(SNAP) $(FLAGS) $(JQ) > $(OUTDIR)/analyze-$(notdir $(BIN)).json
	@printf 'wrote %s (%s bytes)\n' $(OUTDIR)/analyze-$(notdir $(BIN)).json $$(wc -c < $(OUTDIR)/analyze-$(notdir $(BIN)).json)

decompile: workdirs check-args
	@test -n "$(ADDR)" || { echo "usage: make decompile ADDR=0x401e46"; exit 1; }
	$(RUN) rea decompile $(IN) $(ADDR) --provider ghidra $(FLAGS) $(JQ) > $(OUTDIR)/decompile-$(notdir $(BIN))-$(ADDR).json

function: workdirs check-args
	@test -n "$(NAME)" || { echo "usage: make function NAME=main  (NAME may also be an address or 'entry')"; exit 1; }
	$(RUN) rea function $(IN) '$(NAME)' --provider ghidra $(SNAP) $(FLAGS) $(JQ) > $(OUTDIR)/function-$(notdir $(BIN))-$(NAME).json

search: workdirs check-args
	@test -n "$(QUERY)" || { echo "usage: make search QUERY='license'"; exit 1; }
	$(RUN) rea search $(IN) '$(QUERY)' --provider ghidra $(SNAP) $(FLAGS) $(JQ) > $(OUTDIR)/search-$(notdir $(BIN)).json

trace: workdirs check-args
	@test -n "$(QUERY)" || { echo "usage: make trace QUERY='license'"; exit 1; }
	$(RUN) rea trace $(IN) '$(QUERY)' --provider ghidra $(SNAP) $(FLAGS) $(JQ) > $(OUTDIR)/trace-$(notdir $(BIN)).json

xrefs: workdirs check-args
	@test -n "$(ADDR)" || { echo "usage: make xrefs ADDR=0x401e46"; exit 1; }
	$(RUN) rea xrefs $(IN) $(ADDR) --provider ghidra $(FLAGS) $(JQ) > $(OUTDIR)/xrefs-$(notdir $(BIN))-$(ADDR).json

instructions: workdirs check-args
	@test -n "$(ADDR)" || { echo "usage: make instructions ADDR=0x401e46"; exit 1; }
	$(RUN) rea instructions $(IN) $(ADDR) --provider ghidra $(FLAGS) $(JQ) > $(OUTDIR)/instructions-$(notdir $(BIN))-$(ADDR).json

# read-bytes / address-to-file-offset accept no --snapshot (measured: "Unknown flag")
bytes: workdirs check-args
	@test -n "$(ADDR)" || { echo "usage: make bytes ADDR=0x401e46 LENGTH=64"; exit 1; }
	$(RUN) rea read-bytes $(IN) $(ADDR) --length $(LENGTH) $(FLAGS) $(JQ) > $(OUTDIR)/bytes-$(notdir $(BIN))-$(ADDR).json

offset: workdirs check-args
	@test -n "$(ADDR)" || { echo "usage: make offset ADDR=0x401e46"; exit 1; }
	$(RUN) rea address-to-file-offset $(IN) $(ADDR) $(FLAGS) $(JQ) > $(OUTDIR)/offset-$(notdir $(BIN))-$(ADDR).json

pe-resources: workdirs
	$(RUN) rea inspect-pe-resources $(IN) $(FLAGS) $(JQ) > $(OUTDIR)/pe-resources-$(notdir $(BIN)).json

load-image: workdirs
	$(RUN) rea inspect-native-load-image $(IN) $(FLAGS) $(JQ) > $(OUTDIR)/load-image-$(notdir $(BIN)).json

managed-artifact: workdirs
	$(RUN) rea inspect-managed-artifact $(IN) $(FLAGS) $(JQ) > $(OUTDIR)/managed-artifact-$(notdir $(BIN)).json

# paging/summarizing a saved record without starting the engine (measured ~48 ms)
# Measured boundary: inspect_analysis_view accepts records whose parent operation is
# analyze_function, analyze_javascript_application or inspect_binary_layout.
# A `rea analyze` record has parent binary_overview -> kind:"summary" fails closed.
analysis-view: workdirs
	@test -n "$(FILE)" || { echo "usage: make analysis-view FILE=function-<name>.json [FACET=pseudocode] or FILE=analyze-jsapp.json [VIEW_KIND=summary FACET=]"; exit 1; }
	@$(COMPOSE) run --rm -T -e VK="$(VIEW_KIND)" -e FC="$(FACET)" rea sh -c 'e=/work/$(TARGET)/out/envelope-$(FILE); if [ -n "$$FC" ]; then jq -c "{source:{kind:\"inline\",evidence:.},view:{kind:\"$$VK\",facet:\"$$FC\"}}" "/work/$(TARGET)/out/$(FILE)" > "$$e"; else jq -c "{source:{kind:\"inline\",evidence:.},view:{kind:\"$$VK\"}}" "/work/$(TARGET)/out/$(FILE)" > "$$e"; fi; rea inspect-analysis-view "$$e" --json > "/work/$(TARGET)/out/view-$(FILE)"'
	@printf 'wrote %s (%s bytes)\n' $(OUTDIR)/view-$(FILE) $$(wc -c < $(OUTDIR)/view-$(FILE))

evidence:
	@printf '%s\n' \
	 'Boundary (measured): a CLI `--json` result is ONE Evidence record, not a bundle.' \
	 '  rea evidence-export <record.json> <out>  -> fails closed: expects bundle with records+manifest arrays.' \
	 '  rea compare <left> <right>                -> same requirement (bundles only).' \
	 'Bundles come from the MCP tool export_evidence_bundle or from a saved bundle file.' \
	 '' \
	 'Portable paging instead (no engine): make analysis-view FILE=function-<name>.json FACET=pseudocode' \
	 '   (parent operation must be analyze_function / analyze_javascript_application / inspect_binary_layout;' \
	 '    a `rea analyze` record has parent binary_overview, so kind:"summary" fails closed - measured)' \
	 'Or by hand: docker compose -f docker/compose.yaml run --rm -T rea sh -c "jq -c '\''{source:{kind:\"inline\",evidence:.},view:{kind:\"native\",facet:\"pseudocode\"}}'\'' /work/aida64/analysis/function-aida64.unpacked.exe-entry.json | rea inspect-analysis-view --json"' \
	 '' \
	 'Bundle commands (when you actually have a bundle):' \
	 '  rea evidence-import  /work/<target>/analysis/bundle.json' \
	 '  rea evidence-export  /work/<target>/analysis/bundle.json /work/<target>/analysis/canonical.json' \
	 '  rea compare /work/<target>/analysis/left.json /work/<target>/analysis/right.json'

# The container writes as uid 0. Under rootless Docker (measured here) container uid 0
# maps to the daemon-owner host uid, so chowning to 0:0 *inside* the container hands the
# files back to that host user (host then sees owner = your own uid and can write again).
# Chowning to the host uid *inside* the container is wrong: it maps to a different host uid.
# Under a root-run daemon the mapped uid is root, so run `sudo chown -R <you> work` on the host.
chown:
	@$(COMPOSE) run --rm -T rea chown -R 0:0 /work

clean:
	@rm -rf work/aida64/out work/aida64/analysis work/selftest 2>/dev/null || true
	@printf 'removed derived outputs (targets and pinned image untouched)\n'
