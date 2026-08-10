#!/usr/bin/env bash
set -euo pipefail
REF="${OSLC_RM_REF:-master}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/code/oslc-rm-vocab"
mkdir -p "$OUT/raw"
base="https://raw.githubusercontent.com/oslc-op/oslc-specs/${REF}/specs/rm"
files=(requirements-management-vocab.ttl requirements-management-shapes.ttl)
rm -f "$OUT/raw"/*.ttl
items=()
for f in "${files[@]}"; do
  url="$base/$f"
  curl -L --fail --retry 3 --connect-timeout 20 -o "$OUT/raw/$f" "$url"
  sha="$(shasum -a 256 "$OUT/raw/$f" | awk '{print $1}')"
  bytes="$(wc -c < "$OUT/raw/$f" | tr -d ' ')"
  items+=("{ \"file\": \"raw/$f\", \"url\": \"$url\", \"sha256\": \"$sha\", \"bytes\": $bytes }")
done
printf '{\n  "schema": "pnix.ingest.source_receipt.v1",\n  "source_id": "oslc-rm-vocab",\n  "source_name": "OSLC Requirements Management vocabulary metadata",\n  "ref": "%s",\n  "retrieved_at_utc": "%s",\n  "license": "Apache-2.0",\n  "sources": [ %s ],\n  "scope": "requirements-management-vocab.ttl + requirements-management-shapes.ttl structural metadata only",\n  "excluded": ["comments/descriptions", "spec prose", "examples", "live requirements documents", "OSLC service data", "credentials", "execution", "mirror/graph wiring"]\n}\n' "$REF" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$(IFS=,; echo "${items[*]}")" > "$OUT/source-receipt.json"
printf 'updated %s: ttl_files=%s ref=%s\n' "$OUT" "${#files[@]}" "$REF" >&2
