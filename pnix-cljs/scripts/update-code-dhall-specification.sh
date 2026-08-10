#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DST="$ROOT/ingest/code/dhall-specification"
REF="${DHALL_SPEC_REF:-master}"
BASE="https://raw.githubusercontent.com/dhall-lang/dhall-lang/$REF"
mkdir -p "$DST/standard"
FILES=(LICENSE standard/README.md standard/dhall.abnf standard/syntax.md standard/type-inference.md standard/beta-normalization.md standard/imports.md standard/binary.md standard/versioning.md)
for f in "${FILES[@]}"; do
  mkdir -p "$DST/$(dirname "$f")"
  curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/$f" "$BASE/$f"
done
sha256_file() { shasum -a 256 "$1" | awk '{print $1}'; }
{
  echo '{'
  echo '  "schema": "pnix.ingest_source_manifest.v1",'
  echo '  "source_id": "dhall-specification",'
  echo '  "source_name": "Dhall language standard",'
  echo '  "license_id": "BSD-3-Clause",'
  echo "  \"ref\": \"$REF\","
  echo "  \"retrieved_at\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
  echo "  \"source_base_url\": \"$BASE\","
  echo '  "files": ['
  for i in "${!FILES[@]}"; do
    f="${FILES[$i]}"; comma=','; [ "$i" = "$((${#FILES[@]}-1))" ] && comma=''
    printf '    { "path": "%s", "sha256": "%s" }%s\n' "$f" "$(sha256_file "$DST/$f")" "$comma"
  done
  echo '  ],'
  echo '  "policy": "ABNF production metadata + headings only. Prose/examples/literate code/test vectors/tool execution excluded."'
  echo '}'
} > "$DST/source-manifest.json"
printf 'updated %s ref=%s\n' "$DST/source-manifest.json" "$REF"
