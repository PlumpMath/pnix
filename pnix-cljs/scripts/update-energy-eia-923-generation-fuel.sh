#!/usr/bin/env bash
# EIA-923 generation/fuel updater. Downloads latest valid EIA-923 zip; generator stores bounded annual aggregates only.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/energy/eia-923-generation-fuel"
mkdir -p "$DEST"
python3 - "$DEST" <<'PY'
import datetime, hashlib, json, os, pathlib, re, sys, urllib.parse, urllib.request, zipfile, io

dest=pathlib.Path(sys.argv[1])
ua={'User-Agent':'pnix-ingest/1.0 (EIA-923 annual aggregate metadata only)'}
def fetch(url, timeout=180):
    req=urllib.request.Request(url,headers=ua)
    with urllib.request.urlopen(req,timeout=timeout) as r:
        return r.read(), r.geturl(), r.headers.get('content-type','')
page_url='https://www.eia.gov/electricity/data/eia923/'
page_raw, page_final, page_ctype = fetch(page_url, 120)
(dest/'eia923.html').write_bytes(page_raw)
html=page_raw.decode('utf-8','replace')
links=[]
for href in re.findall(r'href=["\']([^"\']*f923_\d{4}\.zip)["\']', html, flags=re.I):
    full=urllib.parse.urljoin(page_url, href)
    m=re.search(r'f923_(\d{4})\.zip', full, flags=re.I)
    if m: links.append((int(m.group(1)), full))
if not links:
    raise SystemExit('no EIA-923 zip links found')
if os.environ.get('EIA923_ZIP_URL'):
    candidates=[(None, os.environ['EIA923_ZIP_URL'])]
else:
    candidates=sorted(links, key=lambda x:x[0], reverse=True)
errors=[]
chosen=None
for year, url in candidates:
    raw, final, ctype = fetch(url)
    if raw[:2] != b'PK':
        errors.append({'year':year,'url':url,'reason':'not-zip','sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw)})
        continue
    try:
        z=zipfile.ZipFile(io.BytesIO(raw)); members=[i.filename for i in z.infolist()]
    except Exception as e:
        errors.append({'year':year,'url':url,'reason':'bad-zip:'+str(e),'sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw)})
        continue
    chosen=(year,url,raw,final,ctype,members)
    break
if not chosen:
    raise SystemExit('no valid EIA-923 zip found: '+json.dumps(errors,ensure_ascii=False))
year, zip_url, zip_raw, zip_final, zip_ctype, members = chosen
(dest/'eia923.zip').write_bytes(zip_raw)
receipt={
 'schema':'pnix.ingest.source_receipt.v1',
 'source':'EIA Form EIA-923 generation and fuel data',
 'version':f'snapshot-{year}' if year else 'snapshot-forced-url',
 'page_url':page_url,
 'page_final_url':page_final,
 'page_sha256':hashlib.sha256(page_raw).hexdigest(),
 'page_content_type':page_ctype,
 'zip_url':zip_url,
 'zip_final_url':zip_final,
 'zip_sha256':hashlib.sha256(zip_raw).hexdigest(),
 'zip_size_bytes':len(zip_raw),
 'zip_content_type':zip_ctype,
 'selected_year':year,
 'skipped_invalid_candidates':errors,
 'zip_member_count':len(members),
 'zip_members':members,
 'retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),
 'license':'EIA public domain / acknowledgment requested',
 'scope':'bounded EIA-923 annual aggregate generation/fuel rows only; monthly vectors, dispatch/control guidance, security-sensitive detail, forecast/trading advice, and graph/mirror wiring excluded'
}
(dest/'source-receipt.json').write_text(json.dumps(receipt,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
print(f'updated EIA-923: {receipt["version"]} bytes={len(zip_raw)} members={len(members)} sha={receipt["zip_sha256"]}')
PY
