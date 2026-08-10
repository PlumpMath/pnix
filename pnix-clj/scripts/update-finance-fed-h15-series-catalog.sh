#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/finance/fed-h15-series-catalog"
mkdir -p "$OUT/raw"
python3 - "$OUT" <<'PY'
import datetime, hashlib, json, pathlib, urllib.parse, urllib.request
out=pathlib.Path(__import__('sys').argv[1])
base='https://www.federalreserve.gov/datadownload/Review.aspx'
packages=[
 {'id':'treasury_constant_maturities','series':'bf17364827e38702b42a58cf8eaa3f78','lastobs':'','label':'Treasury Constant Maturities'},
 {'id':'weekly_fedfunds_prime_discount','series':'8e83f7f17c5cea4d190d85ae6737639f','lastobs':'52','label':'Weekly Averages (Fed Funds, Prime and Discount rates)'},
 {'id':'weekly_averages','series':'c3ec77dedd37c9aa112f71c9eba34b50','lastobs':'52','label':'Weekly Averages'},
 {'id':'monthly_averages','series':'d7e27b7b09a3a7feae95b9c61781fcd8','lastobs':'12','label':'Monthly Averages'}
]
def fetch(url):
    req=urllib.request.Request(url,headers={'User-Agent':'pnix-fed-h15-ingest/1.0'})
    with urllib.request.urlopen(req,timeout=60) as r:
        return r.read(), r.headers.get('content-type')
files=[]
for pkg in packages:
    qs={'filetype':'csv','from':'','label':'include','layout':'seriescolumn','rel':'H15','series':pkg['series'],'to':'','type':'package'}
    if pkg['lastobs']: qs['lastobs']=pkg['lastobs']
    url=base+'?'+urllib.parse.urlencode(qs)
    data,ctype=fetch(url)
    path=f"raw/{pkg['id']}.html"
    dest=out/path; dest.write_bytes(data)
    files.append({**pkg,'path':path,'url':url,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest(),'content_type':ctype})
manifest={'schema':'pnix.source_manifest.v1','source':'Federal Reserve H.15 DDP series catalog pages','retrieved_at':datetime.date.today().isoformat(),'policy':'Review pages only; no direct CSV/XML data payload download','files':files}
(out/'source-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'ok':True,'out':str(out),'packages':len(files)},ensure_ascii=False))
PY
