#!/usr/bin/env bash
# QUDT native core snapshot downloader.
# Host responsibility only: fetch official release zip and copy selected TTL source files.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
VERSION="latest"
OUT="$ROOT/ingest/units/qudt"
while [ $# -gt 0 ]; do
  case "$1" in
    --version) VERSION="$2"; shift 2;;
    --out) OUT="$2"; shift 2;;
    *) echo "unknown arg: $1" >&2; exit 2;;
  esac
done
if [ "$VERSION" = "latest" ]; then
  META="$(mktemp)"
  curl -fsSL https://api.github.com/repos/qudt/qudt-public-repo/releases/latest -o "$META"
  VERSION="$(python3 - "$META" <<'PY'
import json,sys
print(json.load(open(sys.argv[1]))['tag_name'])
PY
)"
  URL="$(python3 - "$META" <<'PY'
import json,sys
j=json.load(open(sys.argv[1]))
assets=j.get('assets',[])
for a in assets:
    if a.get('name','').endswith('.zip'):
        print(a['browser_download_url']); break
else:
    raise SystemExit('no zip asset')
PY
)"
else
  CLEAN="${VERSION#v}"
  URL="https://github.com/qudt/qudt-public-repo/releases/download/$VERSION/qudt-public-repo-$CLEAN.zip"
fi
TMP="$(mktemp -d)"
ZIP="$TMP/qudt.zip"
curl -fL "$URL" -o "$ZIP"
rm -rf "$TMP/src"
mkdir -p "$TMP/src"
unzip -q "$ZIP" -d "$TMP/src"
mkdir -p "$OUT/schema" "$OUT/vocab/unit" "$OUT/vocab/quantitykinds" "$OUT/vocab/dimensionvectors" "$OUT/vocab/prefixes" "$OUT/vocab/constants" "$OUT/vocab/systems"
cp "$TMP/src/LICENSE.md" "$OUT/LICENSE.md"
cp "$TMP/src/README.md" "$OUT/README.md"
cp "$TMP/src/schema/SCHEMA_QUDT.ttl" "$OUT/schema/SCHEMA_QUDT.ttl"
cp "$TMP/src/vocab/unit/VOCAB_QUDT-UNITS-ALL.ttl" "$OUT/vocab/unit/VOCAB_QUDT-UNITS-ALL.ttl"
cp "$TMP/src/vocab/quantitykinds/VOCAB_QUDT-QUANTITY-KINDS-ALL.ttl" "$OUT/vocab/quantitykinds/VOCAB_QUDT-QUANTITY-KINDS-ALL.ttl"
cp "$TMP/src/vocab/dimensionvectors/VOCAB_QUDT-DIMENSION-VECTORS.ttl" "$OUT/vocab/dimensionvectors/VOCAB_QUDT-DIMENSION-VECTORS.ttl"
cp "$TMP/src/vocab/prefixes/VOCAB_QUDT-PREFIXES.ttl" "$OUT/vocab/prefixes/VOCAB_QUDT-PREFIXES.ttl"
cp "$TMP/src/vocab/constants/VOCAB_QUDT-CONSTANTS.ttl" "$OUT/vocab/constants/VOCAB_QUDT-CONSTANTS.ttl"
cp "$TMP/src/vocab/systems/VOCAB_QUDT-SYSTEM-OF-UNITS-ALL.ttl" "$OUT/vocab/systems/VOCAB_QUDT-SYSTEM-OF-UNITS-ALL.ttl"
cp "$TMP/src/vocab/systems/VOCAB_QUDT-SYSTEM-OF-QUANTITY-KINDS-ALL.ttl" "$OUT/vocab/systems/VOCAB_QUDT-SYSTEM-OF-QUANTITY-KINDS-ALL.ttl"
printf '%s\n' "$VERSION" > "$OUT/VERSION"
printf '%s\n' "$URL" > "$OUT/SOURCE_URL"
shasum -a 256 "$ZIP" | awk '{print $1}' > "$OUT/ZIP_SHA256"
(
  cd "$OUT"
  shasum -a 256 LICENSE.md README.md schema/SCHEMA_QUDT.ttl \
    vocab/unit/VOCAB_QUDT-UNITS-ALL.ttl \
    vocab/quantitykinds/VOCAB_QUDT-QUANTITY-KINDS-ALL.ttl \
    vocab/dimensionvectors/VOCAB_QUDT-DIMENSION-VECTORS.ttl \
    vocab/prefixes/VOCAB_QUDT-PREFIXES.ttl \
    vocab/constants/VOCAB_QUDT-CONSTANTS.ttl \
    vocab/systems/VOCAB_QUDT-SYSTEM-OF-UNITS-ALL.ttl \
    vocab/systems/VOCAB_QUDT-SYSTEM-OF-QUANTITY-KINDS-ALL.ttl > SHA256SUMS
)
echo "QUDT snapshot downloaded: version=$VERSION -> $OUT"
