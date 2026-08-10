#!/usr/bin/env bash
# IANA Media Types registry snapshot updater.
# IANA registry는 release tag catalog가 없으므로, 최신 XML snapshot을 받아 content-hash append-only로 누적한다.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
OUTDIR="$ROOT/ingest/registry/iana-media-types"
URL="https://www.iana.org/assignments/media-types/media-types.xml"
mkdir -p "$OUTDIR"
curl -fsSL "$URL" -o "$OUTDIR/media-types.xml"
python3 - "$OUTDIR/manifest.json" "$OUTDIR/media-types.xml" "$URL" <<'PY'
import datetime, hashlib, json, sys
manifest, src, url = sys.argv[1:]
raw = open(src,'rb').read()
obj = {
  'schema': 'pnix.ingest.source_manifest.v1',
  'source_id': 'iana-media-types',
  'project': 'IANA Media Types Registry',
  'snapshot_kind': 'latest-xml-snapshot',
  'source_url': url,
  'source_path': 'media-types.xml',
  'retrieved_at_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
  'source_sha256': hashlib.sha256(raw).hexdigest(),
  'license': 'IANA any-purpose registry terms',
  'version_policy': 'No release tags are exposed. Each fetched XML snapshot is content-addressed and append-only in redb; rerun this script to capture newer registry snapshots.'
}
open(manifest,'w',encoding='utf-8').write(json.dumps(obj, ensure_ascii=False, indent=2)+'\n')
print(json.dumps(obj, ensure_ascii=False))
PY
