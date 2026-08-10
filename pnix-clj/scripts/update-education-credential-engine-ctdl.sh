#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/education/credential-engine-ctdl"
mkdir -p "$OUT/raw"
urls=(
  "https://credreg.net/ctdl/schema/context/json"
  "https://credreg.net/ctdl/schema/encoding/json"
)
rm -f "$OUT/raw"/*.json
items=()
for u in "${urls[@]}"; do
  name="$(basename "$(dirname "$u")")-$(basename "$u").json"
  curl -L --fail --retry 3 --connect-timeout 20 -o "$OUT/raw/$name" "$u"
  sha="$(shasum -a 256 "$OUT/raw/$name" | awk '{print $1}')"
  bytes="$(wc -c < "$OUT/raw/$name" | tr -d ' ')"
  items+=("{ \"url\": \"$u\", \"file\": \"raw/$name\", \"sha256\": \"$sha\", \"bytes\": $bytes }")
done
printf '{\n  "schema": "pnix.ingest.source_receipt.v1",\n  "source_id": "credential-engine-ctdl",\n  "source_name": "Credential Engine CTDL schema metadata",\n  "retrieved_at_utc": "%s",\n  "license": "CC-BY-4.0",\n  "sources": [ %s ],\n  "scope": "official CTDL context JSON + encoding JSON schema term structure only",\n  "excluded": ["rdfs:comment/descriptions", "handbook prose", "guidance", "examples", "Credential Registry records", "credential/person/org data", "API keys", "execution", "mirror/graph wiring"]\n}\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$(IFS=,; echo "${items[*]}")" > "$OUT/source-receipt.json"
printf 'updated %s: json_files=%s\n' "$OUT" "${#urls[@]}" >&2
