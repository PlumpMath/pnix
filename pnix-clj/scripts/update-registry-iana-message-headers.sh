#!/usr/bin/env bash
# IANA Message Headers XML 최신 snapshot 수집. redb 적재는 하지 않는다.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
SRC_DIR="$ROOT/ingest/registry/iana-message-headers"
URL="https://www.iana.org/assignments/message-headers/message-headers.xml"
mkdir -p "$SRC_DIR"
TMP="$SRC_DIR/message-headers.xml.tmp"
OUT="$SRC_DIR/message-headers.xml"
curl -fsSL "$URL" -o "$TMP"
mv "$TMP" "$OUT"
SHA=$(shasum -a 256 "$OUT" | awk '{print $1}')
BYTES=$(wc -c < "$OUT" | tr -d ' ')
RETRIEVED=$(date -u +%Y-%m-%dT%H:%M:%SZ)
cat > "$SRC_DIR/manifest.json" <<JSON
{
  "schema": "pnix.ingest.manifest.v1",
  "source_id": "iana-message-headers",
  "project": "IANA Message Headers",
  "snapshot_kind": "latest-content-addressed",
  "retrieved_at_utc": "$RETRIEVED",
  "license": "IANA any-purpose registry terms",
  "version_policy": "IANA registry has no release tags; update script fetches the latest XML snapshot and redb key is content-addressed by generated pnix source hash.",
  "source_url": "$URL",
  "source_path": "message-headers.xml",
  "source_sha256": "$SHA",
  "source_bytes": $BYTES
}
JSON
printf 'updated %s sha256=%s bytes=%s\n' "$OUT" "$SHA" "$BYTES"
