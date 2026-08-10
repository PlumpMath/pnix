#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DST="$ROOT/ingest/units/bipm-si-brochure"
mkdir -p "$DST"
PAGE="https://www.bipm.org/en/publications/si-brochure"
PDF="https://www.bipm.org/documents/20126/41483022/SI-Brochure-9-EN.pdf"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/si-brochure.html" "$PAGE"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/SI-Brochure-9-EN.pdf" "$PDF"
sha256_file() { shasum -a 256 "$1" | awk '{print $1}'; }
cat > "$DST/source-manifest.json" <<JSON
{
  "schema": "pnix.ingest_source_manifest.v1",
  "source_id": "bipm-si-brochure",
  "source_name": "BIPM SI Brochure",
  "license_id": "CC-BY-4.0",
  "retrieved_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "page_url": "$PAGE",
  "pdf_url": "$PDF",
  "files": [
    { "path": "si-brochure.html", "sha256": "$(sha256_file "$DST/si-brochure.html")" },
    { "path": "SI-Brochure-9-EN.pdf", "sha256": "$(sha256_file "$DST/SI-Brochure-9-EN.pdf")" }
  ],
  "policy": "Generated redb rows contain closed SI code tables only. PDF prose/logo assets are excluded."
}
JSON
printf 'updated %s\n' "$DST/source-manifest.json"
