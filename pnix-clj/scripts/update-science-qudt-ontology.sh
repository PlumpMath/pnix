#!/usr/bin/env bash
set -euo pipefail

# Download/update official QUDT schema/vocabulary RDF/Turtle files.
# Default is latest stable tag found on 2026-06-20; override with QUDT_REF=<tag-or-branch>.

REF="${QUDT_REF:-v3.3.0}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/science/qudt-ontology"
TMP="${TMPDIR:-/tmp}/pnix-qudt-ontology-$$"
URL="https://github.com/qudt/qudt-public-repo/archive/refs/tags/${REF}.tar.gz"
if [[ "$REF" == main || "$REF" == master || "$REF" == */* ]]; then
  URL="https://github.com/qudt/qudt-public-repo/archive/refs/heads/${REF}.tar.gz"
fi

rm -rf "$TMP"
mkdir -p "$TMP" "$OUT/raw"
trap 'rm -rf "$TMP"' EXIT

printf 'QUDT update: ref=%s\n' "$REF" >&2
curl -L --fail --retry 3 --connect-timeout 20 -o "$TMP/qudt.tar.gz" "$URL"
SHA256="$(shasum -a 256 "$TMP/qudt.tar.gz" | awk '{print $1}')"
tar -xzf "$TMP/qudt.tar.gz" -C "$TMP"
SRC="$(find "$TMP" -maxdepth 1 -type d -name 'qudt-public-repo-*' | head -1)"
if [[ -z "$SRC" || ! -d "$SRC/src/main/rdf" ]]; then
  echo "QUDT src/main/rdf not found" >&2
  exit 1
fi

rm -rf "$OUT/raw"
mkdir -p "$OUT/raw"
# Keep official schema/vocab only. Exclude examples, validation, build-derived internals, docs.
while IFS= read -r -d '' f; do
  rel="${f#$SRC/}"
  dest="$OUT/raw/${rel//\//__}"
  cp "$f" "$dest"
done < <(find "$SRC/src/main/rdf/schema" "$SRC/src/main/rdf/vocab" -type f -name '*.ttl' -print0 | sort -z)

cat > "$OUT/source-receipt.json" <<JSON
{
  "schema": "pnix.ingest.source_receipt.v1",
  "source_id": "qudt-ontology",
  "source_name": "QUDT quantity/unit/dimension ontology structural metadata",
  "ref": "${REF}",
  "archive_url": "${URL}",
  "archive_sha256": "${SHA256}",
  "license": "CC-BY-4.0",
  "retrieved_at_utc": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "raw_files": $(find "$OUT/raw" -type f -name '*.ttl' | wc -l | tr -d ' '),
  "scope": "src/main/rdf/schema and src/main/rdf/vocab Turtle structural metadata only",
  "excluded": ["examples", "validation", "build internals", "comments/descriptions/prose", "conversion execution", "mirror/graph wiring"]
}
JSON

printf 'updated %s: ttl_files=%s sha256=%s\n' "$OUT" "$(find "$OUT/raw" -type f -name '*.ttl' | wc -l | tr -d ' ')" "$SHA256" >&2
