#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/accounting/treasury-fiscaldata-api"
BASE="${TREASURY_FISCALDATA_BASE:-https://api.fiscaldata.treasury.gov/services/api/fiscal_service/v2}"
ENDPOINTS=(
  "accounting/od/debt_to_penny"
  "accounting/od/avg_interest_rates"
  "accounting/od/interest_expense"
)
mkdir -p "$OUT"
: > "$OUT/endpoints.txt"
for ep in "${ENDPOINTS[@]}"; do
  safe="${ep//\//__}"
  url="$BASE/$ep?format=json&page[size]=1"
  curl -g -L --fail --retry 3 --retry-delay 2 -o "$OUT/$safe.json.tmp" "$url"
  mv "$OUT/$safe.json.tmp" "$OUT/$safe.json"
  printf '%s\t%s\t%s\n' "$ep" "$url" "$safe.json" >> "$OUT/endpoints.txt"
done
sha256sum "$OUT"/*.json > "$OUT/source-files.sha256"
cat > "$OUT/source-manifest.json" <<JSON
{
  "schema": "pnix.ingest.source_manifest.v1",
  "source_id": "treasury-fiscaldata-api-metadata",
  "base_url": "$BASE",
  "license": "US-PD",
  "retrieved_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "policy": "metadata only; data rows discarded by generator"
}
JSON
echo "updated $OUT endpoints=${#ENDPOINTS[@]}"
