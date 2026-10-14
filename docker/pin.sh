#!/bin/sh
# Digest pins: verify (default) or rewrite (--update) the ARG digest lines in
# docker/Dockerfile. Maintenance-time tool: needs curl + sha256sum (present on
# typical hosts; the toolkit itself runs entirely in Docker).
#
#   sh docker/pin.sh            # compare Dockerfile pins with upstream digests
#   sh docker/pin.sh --update   # write the upstream digests into the Dockerfile
set -eu

DOCKERFILE="${DOCKERFILE:-docker/Dockerfile}"
UPDATE="${1:-}"

arg() { # arg NAME  -> prints the ARG value for NAME
  sed -n "s/^ARG ${1}=//p" "$DOCKERFILE" | head -1
}

set_pin() { # set_pin NAME VALUE  -> rewrites that ARG line
  NAME="$1"; VALUE="$2"
  LINE="ARG ${NAME}=${VALUE}"
  # sed portability: escape / in values (URL-derived digests contain none).
  sed -i "s|^ARG ${NAME}=.*$|${LINE}|" "$DOCKERFILE"
  printf 'pinned %-14s %s\n' "$NAME" "$VALUE"
}

upstream_node() {
  version="$(arg NODE_VERSION)"
  url="$(arg NODE_URL)"
  # Official per-file digests: fetch the release SHASUMS256.txt and take the
  # line whose filename matches the tarball we download.
  name=$(printf '%s\n' "$url" | sed 's|.*/||')
  sums_url=$(printf '%s\n' "$url" | sed 's|/'"$name"$'|/SHASUMS256.txt|')
  curl -fsSL "$sums_url" | grep -- " ${name}\$" | cut -d' ' -f1
}

upstream_jdk() {
  # Adoptium publishes <file>.sha256.txt next to each binary (checksum_link).
  url="$(arg JDK_URL)"
  curl -fsSL "${url}.sha256.txt" | cut -d' ' -f1
}

upstream_ghidra() {
  # Ghidra publishes no checksum asset: stream the asset and hash it.
  curl -fsSL "$(arg GHIDRA_URL)" | sha256sum | cut -d' ' -f1
}

check_one() { # check_one LABEL PIN_VALUE UPSTREAM_VALUE
  label="$1"; pinned="$2"; upstream="$3"
  if [ "$pinned" = "$upstream" ]; then
    printf 'OK    %-6s %s\n' "$label" "$pinned"
    return 0
  fi
  printf 'DRIFT %-6s pinned=%s upstream=%s\n' "$label" "$pinned" "$upstream"
  return 1
}

status=0

node_pinned="$(arg NODE_SHA256)"; node_up="$(upstream_node)"
jdk_pinned="$(arg JDK_SHA256)";  jdk_up="$(upstream_jdk)"

if [ "$UPDATE" = "--update" ]; then
  set_pin NODE_SHA256 "$node_up"
  set_pin JDK_SHA256 "$jdk_up"
  printf 'computing GHIDRA_SHA256 (streaming %s):\n' "$(arg GHIDRA_URL)"
  set_pin GHIDRA_SHA256 "$(upstream_ghidra)"
  printf '\nDockerfile pins refreshed. Rebuild: make build && make selftest\n'
  exit 0
fi

check_one node "$node_pinned" "$node_up" || status=1
check_one jdk "$jdk_pinned" "$jdk_up" || status=1

ghidra_pinned="$(arg GHIDRA_SHA256)"
if [ "$ghidra_pinned" = "PENDING_VERIFY_AND_PIN" ] || [ -z "$ghidra_pinned" ]; then
  printf 'UNPINNED ghidra  upstream=%s\n  paste it into %s (ARG GHIDRA_SHA256) or run: sh docker/pin.sh --update\n' "$(upstream_ghidra)" "$DOCKERFILE"
  status=1
else
  check_one ghidra "$ghidra_pinned" "$(upstream_ghidra)" || status=1
fi

[ "$status" = 0 ] && printf '\nAll digest pins match upstream. Image builds are reproducible.\n'
exit "$status"
