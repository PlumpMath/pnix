#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/text/unicode-ucd"
URL="https://www.unicode.org/Public/UCD/latest/ucd/UCD.zip"
mkdir -p "$OUT"
curl -fL --retry 3 "$URL" -o "$OUT/UCD.zip"
python3 - <<'PY' "$OUT" "$URL"
import hashlib, json, os, pathlib, sys, zipfile
out=pathlib.Path(sys.argv[1]); url=sys.argv[2]; p=out/'UCD.zip'; b=p.read_bytes()
with zipfile.ZipFile(p) as z:
    readme=z.read('ReadMe.txt').decode('utf-8','replace') if 'ReadMe.txt' in z.namelist() else ''
version='unknown'
for line in readme.splitlines():
    if 'Unicode Character Database' in line or 'Version' in line:
        version=line.strip(); break
manifest={
  'schema':'pnix.ingest.manifest.v1',
  'source_id':'unicode-ucd',
  'project':'Unicode Character Database',
  'snapshot_kind':'latest-content-addressed',
  'retrieved_at_utc': os.popen('date -u +%Y-%m-%dT%H:%M:%SZ').read().strip(),
  'license':'Unicode License v3',
  'version_policy':'Update script fetches latest UCD.zip; redb key is content-addressed by generated pnix source hash.',
  'source_url': url,
  'source_path': str(p),
  'source_sha256': hashlib.sha256(b).hexdigest(),
  'source_bytes': len(b),
  'upstream_readme_version_line': version,
}
(out/'manifest.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
print(json.dumps(manifest, ensure_ascii=False, indent=2))
PY
