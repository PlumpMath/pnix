#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/regulatory/ecfr-section-reference"
BASE="${ECFR_BASE:-https://www.ecfr.gov/api/versioner/v1}"
TITLES="${ECFR_SECTION_TITLES:-16 29 14 49 33 46}"
mkdir -p "$OUT"
curl -L --fail --retry 3 --retry-delay 2 -o "$OUT/titles.json.tmp" "$BASE/titles.json"
mv "$OUT/titles.json.tmp" "$OUT/titles.json"
date=$(python3 - "$OUT/titles.json" <<'PY'
import json, sys
d=json.load(open(sys.argv[1]))
print(max(t.get('up_to_date_as_of','') for t in d.get('titles',[]) if t.get('up_to_date_as_of')))
PY
)
: > "$OUT/structures.txt"
for title in $TITLES; do
  url="$BASE/structure/$date/title-$title.json"
  file="title-$title-$date.json"
  curl -L --fail --retry 3 --retry-delay 2 -o "$OUT/$file.tmp" "$url"
  mv "$OUT/$file.tmp" "$OUT/$file"
  printf '%s\t%s\t%s\n' "$title" "$date" "$file" >> "$OUT/structures.txt"
done
sha256sum "$OUT"/*.json > "$OUT/source-files.sha256"
cat > "$OUT/source-manifest.json" <<JSON
{
  "schema": "pnix.ingest.source_manifest.v1",
  "source_id": "ecfr-section-reference",
  "base_url": "$BASE",
  "snapshot_date": "$date",
  "titles": "$TITLES",
  "license": "US-PD",
  "retrieved_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "policy": "section reference metadata only; no XML/full text"
}
JSON
echo "updated $OUT titles=$TITLES snapshot_date=$date"
