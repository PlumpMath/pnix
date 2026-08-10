#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/demography/census-cvap-resource-catalog"
mkdir -p "$OUT/raw"
python3 - "$OUT" <<'PY'
import datetime, hashlib, json, os, pathlib, re, urllib.request
out=pathlib.Path(__import__('sys').argv[1])
base=os.environ.get('CENSUS_CVAP_URL','https://www.census.gov/programs-surveys/decennial-census/about/voting-rights/cvap.html')
years=[y.strip() for y in os.environ.get('CENSUS_CVAP_YEARS','2024,2023,2022,2021,2020').split(',') if y.strip()]
urls=[base]+[f'https://www.census.gov/programs-surveys/decennial-census/about/voting-rights/cvap/{int(y)-4}-{y}-CVAP.html' for y in years]
files=[]
for url in urls:
    req=urllib.request.Request(url,headers={'User-Agent':'pnix-census-cvap-ingest/1.0'})
    with urllib.request.urlopen(req,timeout=60) as r:
        data=r.read(); ctype=r.headers.get('content-type') or ''
    safe=re.sub(r'[^A-Za-z0-9_.-]+','_',url.split('/')[-1] or 'cvap.html')
    if safe in ('cvap.html',''): safe='cvap-overview.html'
    rel='raw/'+safe
    (out/rel).write_bytes(data)
    files.append({'kind':'html_catalog','url':url,'path':rel,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest(),'content_type':ctype})
manifest={'schema':'pnix.source_manifest.v1','source':'Census CVAP pages','retrieved_at':datetime.date.today().isoformat(),'policy':'resource-link catalog metadata only; no CVAP ZIP/PDF/data payload downloads','years':years,'files':files}
(out/'source-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'ok':True,'out':str(out),'files':len(files),'years':years},ensure_ascii=False))
PY
