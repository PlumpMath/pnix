#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DST="$ROOT/ingest/code/toml-specification"
REF="${TOML_SPEC_REF:-}"
if [ -z "$REF" ]; then
  REF="$(curl -L --fail --silent https://api.github.com/repos/toml-lang/toml/releases/latest | python3 -c 'import json,sys; print(json.load(sys.stdin)["tag_name"])')"
fi
BASE="https://raw.githubusercontent.com/toml-lang/toml/$REF"
mkdir -p "$DST"
for f in LICENSE README.md toml.md toml.abnf; do
  curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/$f" "$BASE/$f"
done
sha256_file() { shasum -a 256 "$1" | awk '{print $1}'; }
cat > "$DST/source-manifest.json" <<JSON
{
  "schema": "pnix.ingest_source_manifest.v1",
  "source_id": "toml-specification",
  "source_name": "TOML specification",
  "license_id": "MIT",
  "ref": "$REF",
  "retrieved_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "source_base_url": "$BASE",
  "files": [
    { "path": "LICENSE", "sha256": "$(sha256_file "$DST/LICENSE")" },
    { "path": "README.md", "sha256": "$(sha256_file "$DST/README.md")" },
    { "path": "toml.md", "sha256": "$(sha256_file "$DST/toml.md")" },
    { "path": "toml.abnf", "sha256": "$(sha256_file "$DST/toml.abnf")" }
  ],
  "policy": "ABNF production metadata + headings only. Prose/examples/logos/config corpora excluded."
}
JSON
printf 'updated %s ref=%s\n' "$DST/source-manifest.json" "$REF"
