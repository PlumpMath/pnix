#!/usr/bin/env bash
set -euo pipefail
REF="${CUCUMBER_GHERKIN_REF:-v40.0.0}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/code/cucumber-gherkin"
TMP="${TMPDIR:-/tmp}/pnix-cucumber-gherkin-$$"
URL="https://github.com/cucumber/gherkin/archive/refs/tags/${REF}.tar.gz"
if [[ "$REF" == main || "$REF" == master || "$REF" == */* ]]; then URL="https://github.com/cucumber/gherkin/archive/refs/heads/${REF}.tar.gz"; fi
rm -rf "$TMP"; mkdir -p "$TMP" "$OUT/raw"; trap 'rm -rf "$TMP"' EXIT
curl -L --fail --retry 3 --connect-timeout 20 -o "$TMP/gherkin.tar.gz" "$URL"
SHA256="$(shasum -a 256 "$TMP/gherkin.tar.gz" | awk '{print $1}')"
tar -xzf "$TMP/gherkin.tar.gz" -C "$TMP"
SRC="$(find "$TMP" -maxdepth 1 -type d -name 'gherkin-*' | head -1)"
rm -rf "$OUT/raw"; mkdir -p "$OUT/raw"
cp "$SRC/gherkin.berp" "$OUT/raw/gherkin.berp"
cp "$SRC/gherkin-languages.json" "$OUT/raw/gherkin-languages.json"
cat > "$OUT/source-receipt.json" <<JSON
{
  "schema": "pnix.ingest.source_receipt.v1",
  "source_id": "cucumber-gherkin",
  "source_name": "Cucumber Gherkin grammar and dialect metadata",
  "ref": "${REF}",
  "archive_url": "${URL}",
  "archive_sha256": "${SHA256}",
  "license": "MIT",
  "retrieved_at_utc": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "raw_files": 2,
  "scope": "root gherkin.berp grammar and gherkin-languages.json dialect keyword catalog only",
  "excluded": ["feature examples", "parser source bodies", "generated parsers", "test outputs", "execution", "mirror/graph wiring"]
}
JSON
printf 'updated %s: ref=%s sha256=%s\n' "$OUT" "$REF" "$SHA256" >&2
