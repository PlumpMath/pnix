#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REF="${SBOL_VISUAL_REF:-master}"
OUT="$ROOT/ingest/bio/sbol-visual-catalog"
mkdir -p "$OUT"
receipt="$ROOT/corpus/bio/LICENSES/sbol-visual-catalog.legal-provenance-receipt.json"
bash "$ROOT/scripts/require-legal-provenance-gate.sh" "sbol-visual-catalog" "$receipt"
curl -L --fail --retry 3 --retry-delay 2 -o "$OUT/tree.json" "https://api.github.com/repos/SynBioDex/SBOL-visual/git/trees/${REF}?recursive=1"
curl -L --fail --retry 3 --retry-delay 2 -o "$OUT/sbol-vo.rdf" "https://raw.githubusercontent.com/SynBioDex/SBOL-visual/${REF}/Ontology/v2/sbol-vo.rdf"
curl -L --fail --retry 3 --retry-delay 2 -o "$OUT/LICENSE.html" "https://raw.githubusercontent.com/SynBioDex/SBOL-visual/${REF}/LICENSE.html"
( cd "$OUT" && shasum -a 256 tree.json sbol-vo.rdf LICENSE.html > SHA256SUMS )
printf 'sbol-visual-catalog updated: %s\n' "$OUT"
