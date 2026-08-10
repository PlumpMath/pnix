#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DST="$ROOT/ingest/math/mathml4-schema"
mkdir -p "$DST"
BASE="https://raw.githubusercontent.com/w3c/mathml-schema/main/xsd"
FILES=(mathml4.xsd mathml4-core.xsd mathml4-presentation.xsd mathml4-content.xsd mathml4-strict-content.xsd mathml4-legacy.xsd)
for f in "${FILES[@]}"; do
  curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/$f" "$BASE/$f"
done
sha256_file() { shasum -a 256 "$1" | awk '{print $1}'; }
{
  echo '{'
  echo '  "schema": "pnix.ingest_source_manifest.v1",'
  echo '  "source_id": "mathml4-schema",'
  echo '  "source_name": "W3C MathML 4 schema files",'
  echo '  "license_id": "W3C-Software-Document-License-2023",'
  echo "  \"retrieved_at\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\","
  echo '  "source_base_url": "https://raw.githubusercontent.com/w3c/mathml-schema/main/xsd",'
  echo '  "files": ['
  for i in "${!FILES[@]}"; do
    f="${FILES[$i]}"; comma=','; [ "$i" = "$((${#FILES[@]}-1))" ] && comma=''
    printf '    { "path": "%s", "sha256": "%s" }%s\n' "$f" "$(sha256_file "$DST/$f")" "$comma"
  done
  echo '  ],'
  echo '  "policy": "XSD structure only. Exclude annotation documentation, spec prose, examples, and graph wiring."'
  echo '}'
} > "$DST/source-manifest.json"
printf 'updated %s\n' "$DST/source-manifest.json"
