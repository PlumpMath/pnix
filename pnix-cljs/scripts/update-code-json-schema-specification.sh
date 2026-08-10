#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DST="$ROOT/ingest/code/json-schema-specification"
REF="${JSON_SCHEMA_SPEC_REF:-main}"
BASE="https://raw.githubusercontent.com/json-schema-org/json-schema-spec/$REF"
mkdir -p "$DST"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/rfc8259.txt" "https://www.rfc-editor.org/rfc/rfc8259.txt"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/LICENSE" "$BASE/LICENSE"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/meta.schema.json" "$BASE/specs/meta/meta.schema.json"
sha256_file() { shasum -a 256 "$1" | awk '{print $1}'; }
cat > "$DST/source-manifest.json" <<JSON
{
  "schema": "pnix.ingest_source_manifest.v1",
  "source_id": "json-schema-specification",
  "source_name": "RFC 8259 JSON + JSON Schema meta-schema",
  "license_id": "IETF-RFC + BSD-3-Clause/AFL-3.0",
  "ref": "$REF",
  "retrieved_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "source_base_url": "$BASE",
  "files": [
    { "path": "rfc8259.txt", "sha256": "$(sha256_file "$DST/rfc8259.txt")" },
    { "path": "LICENSE", "sha256": "$(sha256_file "$DST/LICENSE")" },
    { "path": "meta.schema.json", "sha256": "$(sha256_file "$DST/meta.schema.json")" }
  ],
  "policy": "Syntax/schema structural metadata only. Prose/comment/description/example values, test-suite, catalogs, real documents, and graph wiring excluded."
}
JSON
printf 'updated %s ref=%s\n' "$DST/source-manifest.json" "$REF"
