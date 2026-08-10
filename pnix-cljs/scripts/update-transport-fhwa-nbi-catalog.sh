#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DST="$ROOT/ingest/transport/fhwa-nbi-catalog"
mkdir -p "$DST"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/ascii.html" "https://www.fhwa.dot.gov/bridge/nbi/ascii.cfm"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/format.html" "https://www.fhwa.dot.gov/bridge/nbi/format.cfm"
python3 - "$DST" <<'PY'
import json, pathlib, sys, hashlib, datetime
root=pathlib.Path(sys.argv[1])
files=[]
for name in ['ascii.html','format.html']:
    p=root/name; b=p.read_bytes(); files.append({'path':name,'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b)})
(root/'source-manifest.json').write_text(json.dumps({'schema':'pnix.ingest_source_manifest.v1','source_id':'fhwa-nbi-catalog','source_name':'FHWA National Bridge Inventory catalog','license_id':'US-PD / US federal public information','retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':['https://www.fhwa.dot.gov/bridge/nbi/ascii.cfm','https://www.fhwa.dot.gov/bridge/nbi/format.cfm'],'files':files,'policy':'Catalog/link metadata only. Exclude bridge record payloads, coordinates, condition ratings, structural/safety judgments, coding-guide prose bodies, map/geospatial files, routing/operation decisions, graph wiring.'},indent=2,ensure_ascii=False),encoding='utf-8')
print(f'updated {root}/source-manifest.json files={len(files)}')
PY
