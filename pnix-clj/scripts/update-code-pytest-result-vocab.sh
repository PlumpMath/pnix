#!/usr/bin/env bash
set -euo pipefail
REF="${PYTEST_RESULT_VOCAB_REF:-9.1.1}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/code/pytest-result-vocab"
TMP="${TMPDIR:-/tmp}/pnix-pytest-result-vocab-$$"
URL="https://github.com/pytest-dev/pytest/archive/refs/tags/${REF}.tar.gz"
rm -rf "$TMP"; mkdir -p "$TMP" "$OUT/raw"; trap 'rm -rf "$TMP"' EXIT
curl -L --fail --retry 3 --connect-timeout 20 -o "$TMP/pytest.tar.gz" "$URL"
SHA256="$(shasum -a 256 "$TMP/pytest.tar.gz" | awk '{print $1}')"
tar -xzf "$TMP/pytest.tar.gz" -C "$TMP"
SRC="$(find "$TMP" -maxdepth 1 -type d -name 'pytest-*' | head -1)"
rm -rf "$OUT/raw"; mkdir -p "$OUT/raw"
for f in src/_pytest/outcomes.py src/_pytest/reports.py src/_pytest/skipping.py src/_pytest/mark/structures.py; do
  cp "$SRC/$f" "$OUT/raw/${f//\//__}"
done
cat > "$OUT/source-receipt.json" <<JSON
{
  "schema": "pnix.ingest.source_receipt.v1",
  "source_id": "pytest-result-vocab",
  "source_name": "pytest result and mark vocabulary metadata",
  "ref": "${REF}",
  "archive_url": "${URL}",
  "archive_sha256": "${SHA256}",
  "license": "MIT",
  "retrieved_at_utc": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "raw_files": 4,
  "scope": "selected pytest outcome/report/skip-xfail/mark source identifiers only",
  "excluded": ["source bodies", "docstrings/prose", "tests", "test logs", "user results", "configs", "execution", "mirror/graph wiring"]
}
JSON
printf 'updated %s: ref=%s sha256=%s\n' "$OUT" "$REF" "$SHA256" >&2
