#!/usr/bin/env bash
# BIPM SI Digital Framework / SI Reference Point snapshot downloader.
# Host responsibility only: fetch official source files and record commit/license material.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
OUT="$ROOT/ingest/units/bipm-si"
REPO_API="https://api.github.com/repos/TheBIPM/SI_Digital_Framework"
RAW_BASE="https://raw.githubusercontent.com/TheBIPM/SI_Digital_Framework"
BRANCH="${BIPM_SI_REF:-main}"
mkdir -p "$OUT/knowledge_graphs/SI_Reference_Point" "$OUT/knowledge_graphs/quantities"
COMMIT="$(curl -fsSL "$REPO_API/commits/$BRANCH" | python3 -c 'import json,sys; print(json.load(sys.stdin)["sha"])')"
printf '%s\n' "$COMMIT" > "$OUT/COMMIT"
fetch() {
  local path="$1"
  mkdir -p "$OUT/$(dirname "$path")"
  curl -fsSL "$RAW_BASE/$COMMIT/$path" -o "$OUT/$path"
}
fetch LICENCE
fetch README.md
fetch knowledge_graphs/SI_Reference_Point/units.ttl
fetch knowledge_graphs/SI_Reference_Point/prefixes.ttl
fetch knowledge_graphs/SI_Reference_Point/constants.ttl
fetch knowledge_graphs/SI_Reference_Point/si.ttl
fetch knowledge_graphs/quantities/quantities.ttl
(
  cd "$OUT"
  shasum -a 256 COMMIT LICENCE README.md \
    knowledge_graphs/SI_Reference_Point/units.ttl \
    knowledge_graphs/SI_Reference_Point/prefixes.ttl \
    knowledge_graphs/SI_Reference_Point/constants.ttl \
    knowledge_graphs/SI_Reference_Point/si.ttl \
    knowledge_graphs/quantities/quantities.ttl > SHA256SUMS
)
echo "BIPM SI snapshot downloaded: commit=$COMMIT -> $OUT"
