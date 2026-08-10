#!/usr/bin/env bash
# IANA Language Subtag Registry snapshot updater (BCP47).
# release tag 없음: latest registry text snapshot을 content-addressed append-only로 누적한다.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
OUTDIR="$ROOT/ingest/registry/iana-language-subtags"
URL="https://www.iana.org/assignments/language-subtag-registry/language-subtag-registry"
mkdir -p "$OUTDIR"
curl -fsSL "$URL" -o "$OUTDIR/language-subtag-registry.txt"
python3 - "$OUTDIR/manifest.json" "$OUTDIR/language-subtag-registry.txt" "$URL" <<'PY'
import datetime, hashlib, json, re, sys
manifest, src, url = sys.argv[1:]
raw = open(src,'rb').read()
text = raw.decode('utf-8')
m = re.search(r'^File-Date:\s*(.+)$', text, re.M)
obj = {
  'schema': 'pnix.ingest.source_manifest.v1',
  'source_id': 'iana-language-subtags',
  'project': 'IANA Language Subtag Registry',
  'snapshot_kind': 'latest-registry-text-snapshot',
  'source_url': url,
  'source_path': 'language-subtag-registry',
  'file_date': m.group(1).strip() if m else '',
  'retrieved_at_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
  'source_sha256': hashlib.sha256(raw).hexdigest(),
  'license': 'IANA any-purpose registry terms',
  'version_policy': 'No release tags are exposed. File-Date plus content hash identify each snapshot; rerun this script to capture newer registry snapshots.'
}
open(manifest,'w',encoding='utf-8').write(json.dumps(obj, ensure_ascii=False, indent=2)+'\n')
print(json.dumps(obj, ensure_ascii=False))
PY
