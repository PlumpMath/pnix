#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/science/nasa-donki-api-metadata"
mkdir -p "$OUT/raw"
urls=(
  "https://api.nasa.gov/"
  "https://ccmc.gsfc.nasa.gov/tools/DONKI/"
)
rm -f "$OUT/raw"/*.html
items=()
for u in "${urls[@]}"; do
  name="$(echo "$u" | sed 's#[^A-Za-z0-9]#_#g').html"
  path="$OUT/raw/$name"
  curl -L --fail --retry 3 --connect-timeout 20 -o "$path" "$u"
  sha="$(shasum -a 256 "$path" | awk '{print $1}')"
  size="$(wc -c < "$path" | tr -d ' ')"
  items+=("{ \"url\": \"$u\", \"file\": \"raw/$name\", \"sha256\": \"$sha\", \"bytes\": $size }")
done
printf '{\n  "schema": "pnix.ingest.source_receipt.v1",\n  "source_id": "nasa-donki-api-metadata",\n  "source_name": "NASA DONKI API metadata",\n  "retrieved_at_utc": "%s",\n  "license": "USGOV-PUBLIC-METADATA",\n  "sources": [ %s ],\n  "scope": "official public documentation endpoint/query metadata only",\n  "excluded": ["event JSON", "forecasts", "notifications payloads", "model outputs", "API keys", "logos", "prose bodies", "execution", "mirror/graph wiring"]\n}\n' \
  "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$(IFS=,; echo "${items[*]}")" > "$OUT/source-receipt.json"
printf 'updated %s: html_files=%s\n' "$OUT" "${#urls[@]}" >&2
