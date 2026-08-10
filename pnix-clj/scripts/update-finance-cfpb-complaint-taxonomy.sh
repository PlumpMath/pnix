#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/finance/cfpb-complaint-taxonomy"
URL="${CFPB_COMPLAINT_AGG_URL:-https://www.consumerfinance.gov/data-research/consumer-complaints/search/api/v1/?size=0}"
mkdir -p "$OUT"
curl -L --fail --retry 3 --retry-delay 2 -o "$OUT/aggregations.json.tmp" "$URL"
mv "$OUT/aggregations.json.tmp" "$OUT/aggregations.json"
sha256sum "$OUT/aggregations.json" > "$OUT/aggregations.json.sha256"
cat > "$OUT/source-manifest.json" <<JSON
{
  "schema": "pnix.ingest.source_manifest.v1",
  "source_id": "cfpb-consumer-complaint-taxonomy",
  "source_url": "$URL",
  "license": "US-PD",
  "retrieved_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "local_file": "aggregations.json"
}
JSON
echo "updated $OUT"
