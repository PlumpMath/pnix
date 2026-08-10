#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DST="$ROOT/ingest/ml/openml-api-schema"
LIMIT="${OPENML_SCHEMA_LIMIT:-2}"
mkdir -p "$DST"
python3 - "$DST" "$LIMIT" <<'PY'
import json, pathlib, sys, urllib.request, hashlib, datetime
root=pathlib.Path(sys.argv[1]); limit=sys.argv[2]
endpoints={
  'data_list':f'https://www.openml.org/api/v1/json/data/list/limit/{limit}',
  'task_list':f'https://www.openml.org/api/v1/json/task/list/limit/{limit}',
  'run_list':f'https://www.openml.org/api/v1/json/run/list/limit/{limit}',
  'study_list':f'https://www.openml.org/api/v1/json/study/list/limit/{limit}',
}
files=[]
for name,url in endpoints.items():
    req=urllib.request.Request(url,headers={'User-Agent':'pnix-ingest/1.0','Accept':'application/json'})
    with urllib.request.urlopen(req,timeout=45) as r:
        b=r.read()
    # verify parseable
    json.loads(b)
    p=root/f'{name}.json'
    p.write_bytes(b)
    files.append({'path':p.name,'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b),'url':url})
(root/'source-manifest.json').write_text(json.dumps({'schema':'pnix.ingest_source_manifest.v1','source_id':'openml-api-schema','source_name':'OpenML API schema key inventory','license_id':'CC-BY / OpenML public metadata','retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':['https://www.openml.org/apis']+[v for v in endpoints.values()],'limit':int(limit),'files':files,'policy':'Bounded API schema inventory only. Exclude dataset contents, feature values, quality values, task/run/study values, user/person values, names/descriptions/prose, model artifacts, evaluation judgments, graph wiring.'},indent=2,ensure_ascii=False),encoding='utf-8')
print(f'updated {root}/source-manifest.json endpoints={len(endpoints)} limit={limit}')
PY
