#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${USPTO_IDM_DEST:-$ROOT/ingest/trademark/uspto-id-manual}"
UA="${USPTO_IDM_USER_AGENT:-pnix-ingest/0.1 (USPTO ID Manual bounded metadata)}"
LIMIT="${USPTO_IDM_ID_LIMIT:-50}"
mkdir -p "$DEST/raw"
python3 - <<'PY' "$DEST" "$UA" "$LIMIT"
import hashlib,json,pathlib,sys,time,urllib.request,urllib.error
root=pathlib.Path(sys.argv[1]); ua=sys.argv[2]; limit=int(sys.argv[3])
records=[]; errors=[]
for i in range(1,limit+1):
    url=f'https://idm-tmng.uspto.gov/idm2-services/goodServiceTerms/history/{i}'
    req=urllib.request.Request(url,headers={'User-Agent':ua})
    try:
        with urllib.request.urlopen(req,timeout=5) as r:
            data=json.load(r)
        if isinstance(data,list) and data:
            records.extend(data)
    except Exception as e:
        errors.append({'id':i,'error':type(e).__name__})
raw=json.dumps({'limit':limit,'records':records,'errors':errors},ensure_ascii=False,indent=2).encode()
(root/'raw/idm-history-bounded.json').write_bytes(raw)
files=[{'path':'raw/idm-history-bounded.json','url':'https://idm-tmng.uspto.gov/idm2-services/goodServiceTerms/history/{id}','sha256':hashlib.sha256(raw).hexdigest(),'bytes':len(raw)}]
(root/'source-receipt.json').write_text(json.dumps({'schema':'trademark.uspto_id_manual.source_receipt.v1','retrieved_at_unix':int(time.time()),'license':'US-PD / USPTO public taxonomy metadata','id_limit':limit,'record_count':len(records),'error_count':len(errors),'files':files,'excluded':['notes/guidance prose','applicant suggestion emails','actual trademark applications/registrations','legal advice','registrability/infringement/confusion judgments','mirror/graph wiring']},ensure_ascii=False,indent=2)+'\n')
print(f'fetched USPTO ID Manual bounded ids=1..{limit} records={len(records)} errors={len(errors)} into {root}')
PY
