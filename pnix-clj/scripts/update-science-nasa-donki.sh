#!/usr/bin/env bash
# NASA DONKI bounded event metadata snapshot.
# Official api.nasa.gov only. No third-party mirrors. No message/prose/model payload use beyond bounded event refs.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${NASA_DONKI_DEST:-$ROOT/ingest/science/nasa-donki}"
API_KEY="${NASA_API_KEY:-DEMO_KEY}"
START="${NASA_DONKI_START:-2026-06-01}"
END="${NASA_DONKI_END:-2026-06-07}"
BASE="${NASA_DONKI_BASE:-https://api.nasa.gov/DONKI}"
mkdir -p "$DEST/raw"
python3 - "$DEST" "$BASE" "$API_KEY" "$START" "$END" <<'PY'
import datetime as dt, hashlib, json, pathlib, sys, urllib.parse, urllib.request
DEST=pathlib.Path(sys.argv[1]); BASE=sys.argv[2].rstrip('/'); KEY=sys.argv[3]; START=sys.argv[4]; END=sys.argv[5]
endpoints=['CME','GST','IPS','FLR','SEP','MPC','RBE','HSS']
files=[]; failures=[]
for ep in endpoints:
    qs=urllib.parse.urlencode({'startDate':START,'endDate':END,'api_key':KEY})
    url=f'{BASE}/{ep}?{qs}'
    try:
        req=urllib.request.Request(url,headers={'User-Agent':'pnix-nasa-donki-ingest'})
        raw=urllib.request.urlopen(req,timeout=20).read()
        p=DEST/'raw'/f'{ep}.json'; p.write_bytes(raw)
        files.append({'endpoint':ep,'url':url.replace(KEY,'${NASA_API_KEY}'),'relative_path':str(p.relative_to(DEST)),'sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw)})
    except Exception as e:
        failures.append({'endpoint':ep,'error':type(e).__name__+': '+str(e),'url':url.replace(KEY,'${NASA_API_KEY}')})
if not files:
    raise SystemExit('all DONKI endpoints failed: '+json.dumps(failures,ensure_ascii=False))
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'NASA DONKI space weather event API metadata','retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'base_url':BASE,'start_date':START,'end_date':END,'source_urls':['https://api.nasa.gov/',BASE],'license':'NASA public data API / courtesy credit requested','scope':'bounded event refs only; no notification prose, model/time-series payload, mitigation guidance, official-warning replacement, or graph/mirror wiring','files':files,'failures':failures}
(DEST/'source-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2,sort_keys=True)+'\n',encoding='utf-8')
print(f'downloaded NASA DONKI metadata: endpoints_ok={len(files)} failures={len(failures)} -> {DEST}')
PY
