#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/science/kasa-spaceweather-api-catalog"
mkdir -p "$OUT"
UA="pnix-ingest/1.0 (KASA space weather API catalog metadata only; no payload calls)"
BASE="${KASA_SPACEWEATHER_BASE_URL:-https://spaceweather.kasa.go.kr}"
LIST_URL="${KASA_SPACEWEATHER_OPENAPI_LIST_URL:-$BASE/openpotal/datasetInfo/openApiList.do}"
curl -fsSL --retry 2 --max-time 30 -A "$UA" "$LIST_URL" -o "$OUT/openapi-list.html"
python3 - "$OUT" "$BASE" "$LIST_URL" <<'PY'
import re, pathlib, subprocess, sys, json, hashlib, datetime
out=pathlib.Path(sys.argv[1]); base=sys.argv[2].rstrip('/'); list_url=sys.argv[3]
text=(out/'openapi-list.html').read_text(errors='ignore')
codes=sorted(set(re.findall(r"fnDetail\('([^']+)'\)", text)))
for code in codes:
    url=f'{base}/openpotal/datasetInfo/openApiInfo.do?apiCd={code}'
    p=out/f'{code}.html'
    subprocess.run(['curl','-fsSL','--retry','2','--max-time','30','-A','pnix-ingest/1.0',url,'-o',str(p)],check=True)
files=[]
for p in sorted(out.iterdir()):
    if p.name=='source-manifest.json' or not p.is_file(): continue
    files.append({'path':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
manifest={'schema':'pnix.source_manifest.v1','source_id':'kasa-spaceweather-api-catalog','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':[list_url]+[f'{base}/openpotal/datasetInfo/openApiInfo.do?apiCd={c}' for c in codes],'files':files,'policy':'KASA/KSWC Open API catalog metadata only; observation/forecast/warning payloads, prose bodies, credentials, and life-safety guidance excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'api_codes':codes,'files':len(files),'out':str(out)},ensure_ascii=False))
PY
