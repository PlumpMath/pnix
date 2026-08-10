#!/usr/bin/env bash
set -euo pipefail
ROOT="${PNIX_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
OUT_DIR="$ROOT/ingest/math/proof/lean-mathlib4"
API="${LEAN_MATHLIB4_RELEASE_API:-https://api.github.com/repos/leanprover-community/mathlib4/releases/latest}"
mkdir -p "$OUT_DIR"
curl -fsSL "$API" -o "$OUT_DIR/release.json"
python3 - "$OUT_DIR/release.json" > "$OUT_DIR/tarball_url" <<'PY'
import json, sys
j=json.load(open(sys.argv[1]))
print(j.get('tarball_url') or 'https://github.com/leanprover-community/mathlib4/archive/refs/heads/master.tar.gz')
PY
TAG=$(python3 - "$OUT_DIR/release.json" <<'PY'
import json,sys
print(json.load(open(sys.argv[1])).get('tag_name','master'))
PY
)
URL="$(cat "$OUT_DIR/tarball_url")"
TAR="$OUT_DIR/mathlib4-$TAG.tar.gz"
TMP="$TAR.tmp.$$"
echo "download: $URL"
curl -fL --retry 3 --retry-delay 2 "$URL" -o "$TMP"
mv "$TMP" "$TAR"
shasum -a 256 "$TAR" > "$TAR.sha256"
rm -rf "$OUT_DIR/src"
mkdir -p "$OUT_DIR/src"
tar -xzf "$TAR" -C "$OUT_DIR/src" --strip-components=1
{
  echo "source_url=$URL"
  echo "tag=$TAG"
  echo "retrieved_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "sha256=$(cut -d' ' -f1 "$TAR.sha256")"
} > "$TAR.meta"
echo "wrote: $TAR"
cat "$TAR.sha256"
