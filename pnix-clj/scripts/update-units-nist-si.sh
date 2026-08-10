#!/usr/bin/env bash
# NIST SP 330/SP 811 SI publication snapshot downloader.
# Host responsibility only: fetch official NIST pages/PDFs and record hashes.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
OUT="$ROOT/ingest/units/nist-si"
mkdir -p "$OUT"
fetch() { curl -fL "$1" -o "$OUT/$2"; printf '%s\n' "$1" > "$OUT/$2.url"; }
fetch https://www.nist.gov/pml/special-publication-330 sp330.html
fetch https://www.nist.gov/pml/special-publication-330/sp-330-version-history sp330-version-history.html
fetch https://www.nist.gov/pml/special-publication-811 sp811.html
fetch https://www.nist.gov/pml/special-publication-811/nist-guide-si-version-history sp811-version-history.html
fetch https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.330-2019.pdf NIST.SP.330-2019.pdf
fetch https://nvlpubs.nist.gov/nistpubs/Legacy/SP/nistspecialpublication811e2008.pdf nistspecialpublication811e2008.pdf
date -u +%Y-%m-%dT%H:%M:%SZ > "$OUT/RETRIEVED_AT"
(
  cd "$OUT"
  shasum -a 256 sp330.html sp330-version-history.html sp811.html sp811-version-history.html \
    NIST.SP.330-2019.pdf nistspecialpublication811e2008.pdf *.url RETRIEVED_AT > SHA256SUMS
)
echo "NIST SI snapshot downloaded -> $OUT"
