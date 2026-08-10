#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DST="$ROOT/ingest/education/naep-api-metadata"
mkdir -p "$DST"
API_DOC="https://www.nationsreportcard.gov/api_documentation.aspx"
IV_URL="https://www.nationsreportcard.gov/dataservice/getadhocdata.aspx?type=independentvariables&subject=RED&cohort=2&Year=1998,2019"
FAQ_URL="https://www.nationsreportcard.gov/faq.aspx"

curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/api_documentation.html" "$API_DOC"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/independentvariables-red8.json" "$IV_URL"

sha256_file() { shasum -a 256 "$1" | awk '{print $1}'; }
cat > "$DST/source-manifest.json" <<JSON
{
  "schema": "pnix.ingest_source_manifest.v1",
  "source_id": "naep-api-metadata",
  "source_name": "NAEP Data Service API metadata",
  "license_id": "US-PD",
  "retrieved_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "faq_url": "$FAQ_URL",
  "api_documentation_url": "$API_DOC",
  "independentvariables_sample_url": "$IV_URL",
  "files": [
    { "path": "api_documentation.html", "sha256": "$(sha256_file "$DST/api_documentation.html")" },
    { "path": "independentvariables-red8.json", "sha256": "$(sha256_file "$DST/independentvariables-red8.json")" }
  ],
  "policy": "API/code metadata only. Exclude item body, student responses, result values, prose explanations, and graph wiring."
}
JSON
printf 'updated %s\n' "$DST/source-manifest.json"
