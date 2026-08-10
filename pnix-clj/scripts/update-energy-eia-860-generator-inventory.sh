#!/usr/bin/env bash
# EIA-860 generator inventory updater. Downloads latest EIA-860 zip; generator bounds rows downstream.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/energy/eia-860-generator-inventory"
mkdir -p "$DEST"
python3 - "$DEST" <<'PY'
import datetime, hashlib, json, os, pathlib, re, sys, urllib.parse, urllib.request, zipfile, io

dest=pathlib.Path(sys.argv[1])
ua={'User-Agent':'pnix-ingest/1.0 (EIA-860 generator identifier metadata only)'}
def fetch(url):
    req=urllib.request.Request(url,headers=ua)
    with urllib.request.urlopen(req,timeout=180) as r:
        return r.read(), r.geturl(), r.headers.get('content-type','')
page_url='https://www.eia.gov/electricity/data/eia860/'
page_raw, page_final, page_ctype = fetch(page_url)
(dest/'eia860.html').write_bytes(page_raw)
html=page_raw.decode('utf-8','replace')
links=[]
for href in re.findall(r'href=["\']([^"\']*eia860\d{4}(?:ER)?\.zip)["\']', html, flags=re.I):
    full=urllib.parse.urljoin(page_url, href)
    m=re.search(r'eia860(\d{4})(ER)?\.zip', full, flags=re.I)
    if m:
        links.append((int(m.group(1)), 1 if m.group(2) else 0, full))
if not links:
    raise SystemExit('no EIA-860 zip links found')
if os.environ.get('EIA860_ZIP_URL'):
    zip_url=os.environ['EIA860_ZIP_URL']; selected_year=None; early_release=None
else:
    selected_year, early_release, zip_url=max(links, key=lambda x:(x[0],x[1]))
zip_raw, zip_final, zip_ctype = fetch(zip_url)
(dest/'eia860.zip').write_bytes(zip_raw)
try:
    z=zipfile.ZipFile(io.BytesIO(zip_raw))
    members=[i.filename for i in z.infolist()]
except Exception:
    members=[]
receipt={
 'schema':'pnix.ingest.source_receipt.v1',
 'source':'EIA Form EIA-860 generator inventory',
 'version':f'snapshot-{selected_year}-ER' if early_release else (f'snapshot-{selected_year}' if selected_year else 'snapshot-forced-url'),
 'page_url':page_url,
 'page_final_url':page_final,
 'page_sha256':hashlib.sha256(page_raw).hexdigest(),
 'page_content_type':page_ctype,
 'zip_url':zip_url,
 'zip_final_url':zip_final,
 'zip_sha256':hashlib.sha256(zip_raw).hexdigest(),
 'zip_size_bytes':len(zip_raw),
 'zip_content_type':zip_ctype,
 'selected_year':selected_year,
 'early_release':bool(early_release) if selected_year else None,
 'zip_member_count':len(members),
 'zip_members':members,
 'retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),
 'license':'EIA public domain / acknowledgment requested',
 'scope':'bounded EIA-860 generator identifier/classification metadata only; precise location, capacity/operational values, RTO/grid security fields, dispatch/maintenance guidance, forecast/trading advice, and graph/mirror wiring excluded'
}
(dest/'source-receipt.json').write_text(json.dumps(receipt,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
print(f'updated EIA-860: {receipt["version"]} bytes={len(zip_raw)} members={len(members)} sha={receipt["zip_sha256"]}')
PY
