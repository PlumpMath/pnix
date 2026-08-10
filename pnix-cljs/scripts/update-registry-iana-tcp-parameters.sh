#!/usr/bin/env bash
# IANA TCP Parameters XML 최신 snapshot 수집. redb 적재는 하지 않는다.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
SRC_DIR="$ROOT/ingest/registry/iana-tcp-parameters"
URL="https://www.iana.org/assignments/tcp-parameters/tcp-parameters.xml"
mkdir -p "$SRC_DIR"
TMP="$SRC_DIR/tcp-parameters.xml.tmp"
OUT="$SRC_DIR/tcp-parameters.xml"
curl -fsSL "$URL" -o "$TMP"
mv "$TMP" "$OUT"
SHA=$(shasum -a 256 "$OUT" | awk '{print $1}')
BYTES=$(wc -c < "$OUT" | tr -d ' ')
RETRIEVED=$(date -u +%Y-%m-%dT%H:%M:%SZ)
cat > "$SRC_DIR/manifest.json" <<JSON
{
  "schema": "pnix.ingest.manifest.v1",
  "source_id": "iana-tcp-parameters",
  "project": "IANA TCP Parameters",
  "snapshot_kind": "latest-content-addressed",
  "retrieved_at_utc": "$RETRIEVED",
  "license": "IANA any-purpose registry terms",
  "version_policy": "IANA registry has no release tags; update script fetches the latest XML snapshot and redb key is content-addressed by generated pnix source hash.",
  "source_url": "$URL",
  "source_path": "tcp-parameters.xml",
  "source_sha256": "$SHA",
  "source_bytes": $BYTES
}
JSON
printf 'updated %s sha256=%s bytes=%s\n' "$OUT" "$SHA" "$BYTES"
