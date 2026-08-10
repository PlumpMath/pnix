#!/usr/bin/env bash
# IANA Link Relation Types XML snapshot downloader. No graph/mirror/math wiring.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
OUT_DIR="$ROOT/ingest/registry/iana-link-relations"
URL="https://www.iana.org/assignments/link-relations/link-relations.xml"
mkdir -p "$OUT_DIR"
curl -fsSL "$URL" -o "$OUT_DIR/link-relations.xml"
SHA=$(shasum -a 256 "$OUT_DIR/link-relations.xml" | awk '{print $1}')
python3 - "$OUT_DIR/manifest.json" "$URL" "$SHA" <<'PY'
import json, sys
from datetime import datetime, timezone
out,url,sha=sys.argv[1:]
json.dump({
  "schema":"pnix.ingest.source_manifest.v1",
  "source_id":"iana-link-relations",
  "project":"IANA Link Relation Types Registry",
  "snapshot_kind":"latest-xml-snapshot",
  "retrieved_at_utc":datetime.now(timezone.utc).isoformat(),
  "license":"IANA any-purpose registry terms",
  "version_policy":"No release tags are exposed. Each fetched XML snapshot is content-addressed and append-only in redb; rerun this script to capture newer registry snapshots.",
  "source_url":url,
  "source_path":"link-relations.xml",
  "source_sha256":sha
}, open(out,'w',encoding='utf-8'), ensure_ascii=False, indent=2)
print()
PY
printf 'downloaded %s\nsha256=%s\n' "$URL" "$SHA"
