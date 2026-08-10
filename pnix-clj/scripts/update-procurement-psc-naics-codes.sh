#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/procurement/psc-naics-codes"
mkdir -p "$OUT/raw"
python3 - "$OUT" <<'PY'
import datetime, hashlib, html, json, pathlib, re, sys, urllib.parse, urllib.request
out=pathlib.Path(sys.argv[1])
ua='pnix-psc-naics-ingest/1.0'
psc_page=__import__('os').environ.get('PSC_MANUAL_PAGE_URL','https://www.acquisition.gov/psc-manual/all')
naics_year=__import__('os').environ.get('NAICS_YEAR','2022')
naics_url=__import__('os').environ.get('NAICS_STRUCTURE_URL',f'https://www.census.gov/naics/{naics_year}NAICS/{naics_year}_NAICS_Structure.xlsx')

def fetch(url):
    req=urllib.request.Request(url,headers={'User-Agent':ua})
    with urllib.request.urlopen(req,timeout=90) as r:
        return r.read(), r.headers.get('content-type') or ''
page,ct=fetch(psc_page)
(out/'raw'/'psc-manual-all.html').write_bytes(page)
text=page.decode('utf-8','replace')
links=[]
for m in re.finditer(r'href="([^"]+)"[^>]*>(.*?)</a>', text, re.S|re.I):
    href=html.unescape(m.group(1)); label=html.unescape(re.sub(r'\s+',' ',re.sub('<[^>]+>',' ',m.group(2)))).strip()
    if re.search(r'PSC\s+April\s+\d{4}\.xlsx', label, re.I):
        links.append((label,urllib.parse.urljoin(psc_page,href)))
if not links:
    raise SystemExit('no PSC April YYYY xlsx link found')
links.sort(key=lambda x: x[0], reverse=True)
psc_label,psc_url=links[0]
files=[]
for kind,url,name in [('psc_xlsx',psc_url,'psc-current.xlsx'),('naics_xlsx',naics_url,'naics-structure.xlsx')]:
    data,ctype=fetch(url)
    rel='raw/'+name
    (out/rel).write_bytes(data)
    files.append({'kind':kind,'label':psc_label if kind=='psc_xlsx' else f'{naics_year} NAICS Structure','url':url,'path':rel,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest(),'content_type':ctype})
manifest={'schema':'pnix.source_manifest.v1','source':'PSC Manual XLSX + Census NAICS Structure XLSX','retrieved_at':datetime.date.today().isoformat(),'policy':'code/title/date metadata only; manual prose, contract award rows, statistical payloads and graph wiring excluded','psc_page_url':psc_page,'naics_year':naics_year,'files':files}
(out/'source-manifest.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'ok':True,'out':str(out),'files':len(files),'psc_label':psc_label,'naics_year':naics_year},ensure_ascii=False))
PY
