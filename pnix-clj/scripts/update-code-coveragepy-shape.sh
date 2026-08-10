#!/usr/bin/env bash
set -euo pipefail
REF="${COVERAGEPY_SHAPE_REF:-7.14.1}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/code/coveragepy-shape"
TMP="${TMPDIR:-/tmp}/pnix-coveragepy-shape-$$"
URL="https://github.com/nedbat/coveragepy/archive/refs/tags/${REF}.tar.gz"
rm -rf "$TMP"; mkdir -p "$TMP" "$OUT/raw"; trap 'rm -rf "$TMP"' EXIT
curl -L --fail --retry 3 --connect-timeout 20 -o "$TMP/coveragepy.tar.gz" "$URL"
SHA256="$(shasum -a 256 "$TMP/coveragepy.tar.gz" | awk '{print $1}')"
tar -xzf "$TMP/coveragepy.tar.gz" -C "$TMP"
SRC="$(find "$TMP" -maxdepth 1 -type d -name 'coveragepy-*' | head -1)"
rm -rf "$OUT/raw"; mkdir -p "$OUT/raw"
for f in coverage/jsonreport.py coverage/xmlreport.py coverage/html.py coverage/lcovreport.py coverage/report.py coverage/report_core.py coverage/results.py coverage/sqldata.py coverage/types.py; do
  cp "$SRC/$f" "$OUT/raw/${f//\//__}"
done
cat > "$OUT/source-receipt.json" <<JSON
{
  "schema": "pnix.ingest.source_receipt.v1",
  "source_id": "coveragepy-shape",
  "source_name": "coverage.py report/data shape metadata",
  "ref": "${REF}",
  "archive_url": "${URL}",
  "archive_sha256": "${SHA256}",
  "license": "Apache-2.0",
  "retrieved_at_utc": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "raw_files": 9,
  "scope": "selected coverage.py report/data module structural identifiers only",
  "excluded": ["source bodies", "docstrings/prose", "coverage reports", "measured source paths", "line data", "test logs", "execution", "mirror/graph wiring"]
}
JSON
printf 'updated %s: ref=%s sha256=%s\n' "$OUT" "$REF" "$SHA256" >&2
