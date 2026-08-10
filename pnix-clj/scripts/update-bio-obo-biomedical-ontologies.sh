#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/ingest/bio/obo-biomedical-ontologies"
mkdir -p "$OUT"
receipt="$ROOT/corpus/bio/LICENSES/obo-biomedical-ontologies.legal-provenance-receipt.json"
bash "$ROOT/scripts/require-legal-provenance-gate.sh" "obo-biomedical-ontologies" "$receipt"
cat > "$OUT/sources.tsv" <<'EOF'
doid	https://purl.obolibrary.org/obo/doid.obo
hp	https://purl.obolibrary.org/obo/hp.obo
so	https://purl.obolibrary.org/obo/so.obo
EOF
while IFS=$'\t' read -r id url; do
  curl -L --fail --retry 3 --retry-delay 2 -o "$OUT/${id}.obo" "$url"
done < "$OUT/sources.tsv"
( cd "$OUT" && shasum -a 256 *.obo sources.tsv > SHA256SUMS )
printf 'obo-biomedical-ontologies updated: %s\n' "$OUT"
