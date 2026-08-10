#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
YEAR="${MESH_YEAR:-2026}"
OUT="$ROOT/ingest/clinical/mesh-core"
mkdir -p "$OUT"
base="https://nlmpubs.nlm.nih.gov/projects/mesh/MESH_FILES/xmlmesh"
receipt="$ROOT/corpus/clinical/LICENSES/mesh-core.legal-provenance-receipt.json"
bash "$ROOT/scripts/require-legal-provenance-gate.sh" "mesh-core" "$receipt"
curl -L --fail --retry 3 --retry-delay 2 -o "$OUT/desc${YEAR}.gz" "$base/desc${YEAR}.gz"
curl -L --fail --retry 3 --retry-delay 2 -o "$OUT/qual${YEAR}.xml" "$base/qual${YEAR}.xml"
( cd "$OUT" && shasum -a 256 "desc${YEAR}.gz" "qual${YEAR}.xml" > SHA256SUMS )
printf 'mesh-core updated: %s\n' "$OUT"
