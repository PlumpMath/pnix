#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DST="$ROOT/ingest/ml/nist-ai-rmf"
mkdir -p "$DST"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/playbook.html" "https://airc.nist.gov/airmf-resources/playbook/"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/playbook.json" "https://airc.nist.gov/docs/playbook.json"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/playbook.csv" "https://airc.nist.gov/docs/playbook.csv"
python3 - "$DST" <<'PY'
import json, pathlib, sys, hashlib, datetime, csv
root=pathlib.Path(sys.argv[1])
data=json.loads((root/'playbook.json').read_text())
files=[]
for name in ['playbook.html','playbook.json','playbook.csv']:
    p=root/name; b=p.read_bytes(); files.append({'path':name,'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b)})
(root/'source-manifest.json').write_text(json.dumps({'schema':'pnix.ingest_source_manifest.v1','source_id':'nist-ai-rmf','source_name':'NIST AI RMF Playbook','license_id':'US-PD / NIST public information','retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':['https://airc.nist.gov/airmf-resources/playbook/','https://airc.nist.gov/docs/playbook.json','https://airc.nist.gov/docs/playbook.csv'],'records':len(data),'files':files,'policy':'Structural function/category/subcategory/actor/topic metadata only. Exclude prose bodies, suggested actions, documentation questions, reference body text, PDF/XLSX bodies, compliance/risk advice, graph wiring.'},indent=2,ensure_ascii=False),encoding='utf-8')
print(f'updated {root}/source-manifest.json records={len(data)}')
PY
