#!/usr/bin/env bash
# MathML/OpenMath metadata snapshot downloader.
# Host responsibility only: fetch official schema/CD source files and record hashes.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
OUT="$ROOT/ingest/math/mathml-openmath"
mkdir -p "$OUT/mathml/rnc" "$OUT/openmath/cd/Official"
# MathML schema commit
MATHML_META="$(mktemp)"
curl -fsSL https://api.github.com/repos/w3c/mathml-schema/commits/main -o "$MATHML_META"
MATHML_COMMIT="$(python3 - "$MATHML_META" <<'PY'
import json,sys
print(json.load(open(sys.argv[1]))['sha'])
PY
)"
printf '%s\n' "$MATHML_COMMIT" > "$OUT/mathml/COMMIT"
for f in mathml4.rnc mathml4-core.rnc mathml4-content.rnc mathml4-strict-content.rnc mathml4-presentation.rnc mathml4-legacy.rnc; do
  curl -fsSL "https://raw.githubusercontent.com/w3c/mathml-schema/$MATHML_COMMIT/rnc/$f" -o "$OUT/mathml/rnc/$f"
done
# OpenMath official CDs commit and files.
OM_META="$(mktemp)"
curl -fsSL https://api.github.com/repos/OpenMath/CDs/commits/master -o "$OM_META"
OM_COMMIT="$(python3 - "$OM_META" <<'PY'
import json,sys
print(json.load(open(sys.argv[1]))['sha'])
PY
)"
printf '%s\n' "$OM_COMMIT" > "$OUT/openmath/COMMIT"
curl -fsSL "https://raw.githubusercontent.com/OpenMath/CDs/$OM_COMMIT/README.md" -o "$OUT/openmath/README.md"
LIST="$(mktemp)"
curl -fsSL "https://api.github.com/repos/OpenMath/CDs/contents/cd/Official?ref=$OM_COMMIT" -o "$LIST"
python3 - "$LIST" "$OUT/openmath/cd/Official" <<'PY'
import json, os, sys, urllib.request
lst=json.load(open(sys.argv[1])); out=sys.argv[2]
for x in lst:
    name=x.get('name','')
    url=x.get('download_url')
    if name.endswith('.ocd') and url:
        urllib.request.urlretrieve(url, os.path.join(out, name))
PY
date -u +%Y-%m-%dT%H:%M:%SZ > "$OUT/RETRIEVED_AT"
(
  cd "$OUT"
  find mathml openmath -type f | sort | xargs shasum -a 256 > SHA256SUMS
  shasum -a 256 RETRIEVED_AT >> SHA256SUMS
)
echo "MathML/OpenMath snapshot downloaded -> $OUT"
