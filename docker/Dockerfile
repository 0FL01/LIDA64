# nutcracker reverse-engineering toolkit
# Everything REA needs lives here: Node.js, JDK, Ghidra (headless), rea-agents, triage tools.
# Host requirement: Docker + Compose v2 only. Nothing RE-related is installed on the host.
#
# Single source of truth for versions = the ARG pin block below.
# Bump procedure: edit a pin -> `make build` -> `make selftest` (must be green).
# Every download is digest-verified; a mismatch fails the build.

FROM debian:trixie-slim
# Reproducibility note: record the observed base digest here when you bump the base:
#   debian:trixie-slim sha256:<observed-at-first-build>

# ------------------------------ pins ------------------------------
ARG NODE_VERSION=24.18.0
ARG NODE_URL=https://nodejs.org/dist/v24.18.0/node-v24.18.0-linux-x64.tar.xz
# From https://nodejs.org/dist/v24.18.0/SHASUMS256.txt (node-v24.18.0-linux-x64.tar.xz)
ARG NODE_SHA256=55aa7153f9d88f28d765fcdad5ae6945b5c0f98a36881703817e4c450fa76742

ARG JDK_VERSION=21.0.12.1+1
ARG JDK_URL=https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.12.1%2B1/OpenJDK21U-jdk_x64_linux_hotspot_21.0.12.1_1.tar.gz
# From the published Adoptium checksum_link for the same file (<url>.sha256.txt)
ARG JDK_SHA256=ce79869e1307ed8ee1e2baa86a412b1eb5b75d10a01006d788a6f968bcfaee94

ARG GHIDRA_VERSION=12.1.4
ARG GHIDRA_URL=https://github.com/NationalSecurityAgency/ghidra/releases/download/Ghidra_12.1.4_build/ghidra_12.1.4_PUBLIC_20260921.zip
# Mirror fallback:
#   https://sourceforge.net/projects/ghidra.mirror/files/Ghidra_12.1.4_build/ghidra_12.1.4_PUBLIC_20260921.zip/download
# Ghidra publishes no checksum asset -> verify-and-pin: PENDING makes the build print
# the digest of the downloaded archive and stop; paste it here (or `make pin`) and rebuild.
ARG GHIDRA_SHA256=ddac49f903da9d5bac833e5cc79395098b9c33cfd3279be5f31bd00387d2d4db

ARG REA_PKG_VERSION=6.4.0

# ------------------------- triage tools -------------------------
# procps: `ps` is a REA prerequisite for supervised provider launches.
# binutils/file: quick PE/ELF triage (strings/objdump/file) - REA does not need them.
# upx: AIDA64 ships standard-UPX-packed PE binaries; REA has no unpacker.
# tar/gzip/xz-utils/unzip: extracting the pinned archives below.
RUN apt-get update \
  && apt-get install -y --no-install-recommends \
      ca-certificates curl \
      tar gzip xz-utils unzip \
      procps binutils file jq upx \
  && rm -rf /var/lib/apt/lists/*

# ----------------------------- Node.js -----------------------------
# Pinned digest of the official linux-x64 tarball (node/npm/npx ship inside it).
RUN set -eux; \
  curl -fsSL -o /tmp/node.tar.xz ${NODE_URL}; \
  sha256sum /tmp/node.tar.xz | grep -q "^${NODE_SHA256}"; \
  tar -xJf /tmp/node.tar.xz -C /tmp; \
  mv /tmp/node-v${NODE_VERSION}-linux-x64 /opt/node; \
  rm -f /tmp/node.tar.xz; \
  test -x /opt/node/bin/node; \
  /opt/node/bin/node --version
ENV PATH=/opt/node/bin:${PATH}

# --------------------------- JDK (Temurin) ---------------------------
# REA requires a full 64-bit JDK (it looks for javac) and Ghidra compiles REA's
# Java bridge sources at run time - so a JDK, not a headless JRE, is mandatory.
RUN set -eux; \
  curl -fsSL -o /tmp/jdk.tar.gz ${JDK_URL}; \
  sha256sum /tmp/jdk.tar.gz | grep -q "^${JDK_SHA256}"; \
  tar -xzf /tmp/jdk.tar.gz -C /tmp; \
  mv /tmp/jdk-${JDK_VERSION} /opt/jdk; \
  rm -f /tmp/jdk.tar.gz; \
  test -x /opt/jdk/bin/java; \
  test -x /opt/jdk/bin/javac
ENV JAVA_HOME=/opt/jdk
ENV PATH=/opt/jdk/bin:${PATH}

# ------------------------------ Ghidra ------------------------------
# REA accepts Ghidra 12.1.x plus the Java range the install declares
# (Ghidra/application.properties: application.java.min/max). JDK 21 is REA's
# verified pairing for 12.1.4. Install root stays absolute and version-neutral.
RUN set -eux; \
  curl -fsSL -o /tmp/ghidra.zip ${GHIDRA_URL}; \
  if [ -z ${GHIDRA_SHA256+x} ] || [ ${GHIDRA_SHA256} = PENDING_VERIFY_AND_PIN ]; then \
    echo GHIDRA_SHA256 is not pinned. Digest of the downloaded archive: >&2; \
    sha256sum /tmp/ghidra.zip >&2; \
    echo Paste it into docker/Dockerfile ARG GHIDRA_SHA256 and rebuild. >&2; \
    exit 1; \
  fi; \
  sha256sum /tmp/ghidra.zip | grep -q "^${GHIDRA_SHA256}"; \
  mkdir -p /opt/ghidra-stage; \
  unzip -q /tmp/ghidra.zip -d /opt/ghidra-stage; \
  mv /opt/ghidra-stage/ghidra_*_PUBLIC /opt/ghidra; \
  rmdir /opt/ghidra-stage; \
  rm -f /tmp/ghidra.zip; \
  chmod +x /opt/ghidra/support/analyzeHeadless; \
  chmod +x /opt/ghidra/Ghidra/Features/Decompiler/os/linux_x86_64/* 2>/dev/null || true; \
  test -x /opt/ghidra/support/analyzeHeadless; \
  test -x /opt/ghidra/Ghidra/Features/Decompiler/os/linux_x86_64/decompile; \
  grep -E ^application. /opt/ghidra/Ghidra/application.properties; \
  grep -E ^application.version=${GHIDRA_VERSION}$ /opt/ghidra/Ghidra/application.properties
ENV GHIDRA_INSTALL_DIR=/opt/ghidra

# ------------------------------ REA CLI ------------------------------
# The published package ships dist/ plus the Ghidra Java bridge sources, so no
# source build (turbo/typescript/rolldown) is needed. --ignore-scripts keeps the
# install deterministic and skips the optional native Frida binding (runtime
# hooking is out of scope here: static analysis only).
RUN set -eux; \
  npm install --global --ignore-scripts rea-agents@${REA_PKG_VERSION}; \
  test -x /opt/node/bin/rea

# ripgrep for fast scanning of strings/disasm/JSON output. Placed AFTER the pinned
# tool layers so that changing this line does not invalidate the Node/JDK/Ghidra/REA
# cache entries (apt lists were deleted earlier, so update again in this layer).
RUN set -eux; \
  apt-get update; \
  apt-get install -y --no-install-recommends ripgrep; \
  rm -rf /var/lib/apt/lists/*; \
  rg --version | head -1

# Supported Ghidra memory knob: REA reads GHIDRA_HEADLESS_MAXMEM first, then
# GHIDRA_MAXMEM, then defaults to 2G. Do NOT set _JAVA_OPTIONS / JAVA_TOOL_OPTIONS:
# REA clears those on the child JVM it launches.
ENV GHIDRA_HEADLESS_MAXMEM=2G
ENV TMPDIR=/tmp
ENV REA_LOG_LEVEL=info

# ------------------- image self-proof (also `make selftest`) -------------------
COPY selftest.sh /usr/local/selftest.sh
RUN chmod +x /usr/local/selftest.sh
RUN SELFTEST_OUT=/tmp/selftest /usr/local/selftest.sh

WORKDIR /work
CMD ["rea", "--help"]
