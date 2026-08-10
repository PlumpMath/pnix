#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DST="$ROOT/ingest/license/clearlydefined-definitions"
PAGES="${CLEARLYDEFINED_PAGES:-2}"
mkdir -p "$DST/pages"
python3 - "$DST" "$PAGES" <<'PY'
import json, pathlib, sys, hashlib, datetime, urllib.parse, urllib.request
root=pathlib.Path(sys.argv[1]); pages=int(sys.argv[2])
base='https://api.clearlydefined.io/definitions'
token=None; files=[]; total=0
for i in range(pages):
    url=base if not token else base+'?continuationToken='+urllib.parse.quote(token)
    req=urllib.request.Request(url,headers={'User-Agent':'pnix-ingest/1.0','Accept':'application/json'})
    with urllib.request.urlopen(req,timeout=45) as r:
        b=r.read()
    j=json.loads(b)
    out=root/'pages'/f'page-{i+1:03d}.json'
    out.write_bytes(b)
    files.append({'path':str(out.relative_to(root)),'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b),'url':url})
    total+=len(j.get('data') or [])
    token=j.get('continuationToken')
    if not token: break
(root/'source-manifest.json').write_text(json.dumps({'schema':'pnix.ingest_source_manifest.v1','source_id':'clearlydefined-definitions','source_name':'ClearlyDefined definitions API','license_id':'CC0-1.0 curated data','retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':[base,'https://docs.clearlydefined.io/docs/get-involved/using-data'],'pages_requested':pages,'pages_downloaded':len(files),'records':total,'next_continuation_token':token or '','files':files,'policy':'Bounded definition metadata only. Exclude file-level findings, notices, source archives, package artifacts, legal interpretation, compliance advice, final judgment, graph wiring.'},indent=2,ensure_ascii=False),encoding='utf-8')
print(f'updated {root}/source-manifest.json pages={len(files)} records={total} next_token={bool(token)}')
PY
