#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DST="$ROOT/ingest/code/cue-specification"
REF="${CUE_SPEC_REF:-master}"
BASE="https://raw.githubusercontent.com/cue-lang/cue/$REF"
mkdir -p "$DST"
for f in LICENSE doc/ref/spec.md doc/ref/impl.md; do
  mkdir -p "$DST/$(dirname "$f")"
  curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/$f" "$BASE/$f"
done
sha256_file() { shasum -a 256 "$1" | awk '{print $1}'; }
cat > "$DST/source-manifest.json" <<JSON
{
  "schema": "pnix.ingest_source_manifest.v1",
  "source_id": "cue-specification",
  "source_name": "CUE language specification",
  "license_id": "Apache-2.0",
  "ref": "$REF",
  "retrieved_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "source_base_url": "$BASE",
  "files": [
    { "path": "LICENSE", "sha256": "$(sha256_file "$DST/LICENSE")" },
    { "path": "doc/ref/spec.md", "sha256": "$(sha256_file "$DST/doc/ref/spec.md")" },
    { "path": "doc/ref/impl.md", "sha256": "$(sha256_file "$DST/doc/ref/impl.md")" }
  ],
  "policy": "EBNF production metadata + headings only. Prose/examples/tool execution excluded."
}
JSON
printf 'updated %s ref=%s\n' "$DST/source-manifest.json" "$REF"
