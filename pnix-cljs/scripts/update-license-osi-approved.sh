#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DST="$ROOT/ingest/license/osi-approved"
mkdir -p "$DST"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/licenses.json" "https://opensource.org/api/license"
python3 - "$DST" <<'PY'
import json, pathlib, sys, hashlib, datetime
root=pathlib.Path(sys.argv[1])
p=root/'licenses.json'; b=p.read_bytes(); data=json.loads(b)
(root/'source-manifest.json').write_text(json.dumps({'schema':'pnix.ingest_source_manifest.v1','source_id':'osi-approved','source_name':'OSI Approved License API','license_id':'OSI public API metadata','retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':['https://opensource.org/api/license'],'files':[{'path':'licenses.json','sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b)}],'records':len(data),'policy':'API metadata rows only. Exclude full license text, legal interpretation, compatibility/compliance advice, package-specific decisions, graph wiring.'},indent=2,ensure_ascii=False),encoding='utf-8')
print(f'updated {root}/source-manifest.json records={len(data)}')
PY
