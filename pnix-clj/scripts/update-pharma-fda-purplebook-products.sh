#!/usr/bin/env bash
# FDA Purple Book product metadata updater. Latest monthly CSV, bounded downstream by generator.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/pharma/fda-purplebook-products"
mkdir -p "$DEST"
python3 - "$DEST" <<'PY'
import datetime, hashlib, json, pathlib, re, sys, urllib.parse, urllib.request

dest=pathlib.Path(sys.argv[1])
ua={'User-Agent':'pnix-ingest/1.0 (FDA Purple Book product metadata only)'}
def fetch(url):
    req=urllib.request.Request(url,headers=ua)
    with urllib.request.urlopen(req,timeout=120) as r:
        return r.read(), r.geturl(), r.headers.get('content-type','')

downloads_url='https://purplebooksearch.fda.gov/downloads'
html_raw, html_final, html_ctype = fetch(downloads_url)
(dest/'downloads.html').write_bytes(html_raw)
html=html_raw.decode('utf-8','replace')
month_order={m:i for i,m in enumerate(['january','february','march','april','may','june','july','august','september','october','november','december'],1)}
links=[]
for href in re.findall(r'href=["\']([^"\']+\.csv)["\']', html, flags=re.I):
    full=urllib.parse.urljoin(downloads_url, href)
    m=re.search(r'/PurpleBook/(\d{4})/purplebook-search-([A-Za-z]+)-data-download\.csv', full, flags=re.I)
    if not m: continue
    year=int(m.group(1)); month=m.group(2).lower(); mi=month_order.get(month)
    if mi: links.append((year,mi,month,full))
if not links:
    raise SystemExit('no Purple Book CSV links found')
forced_url=None
# env is intentionally read from inherited process through Python os only if present.
import os
if os.environ.get('PURPLEBOOK_CSV_URL'):
    chosen=(0,0,'forced',os.environ['PURPLEBOOK_CSV_URL'])
else:
    chosen=max(links, key=lambda x:(x[0],x[1]))
year, month_i, month_name, csv_url=chosen
csv_raw, csv_final, csv_ctype = fetch(csv_url)
(dest/'purplebook.csv').write_bytes(csv_raw)
receipt={
 'schema':'pnix.ingest.source_receipt.v1',
 'source':'FDA Purple Book monthly downloadable product data',
 'version':f'snapshot-{year}-{month_i:02d}' if year else 'snapshot-forced-url',
 'downloads_url':downloads_url,
 'downloads_final_url':html_final,
 'downloads_sha256':hashlib.sha256(html_raw).hexdigest(),
 'downloads_content_type':html_ctype,
 'csv_url':csv_url,
 'csv_final_url':csv_final,
 'csv_sha256':hashlib.sha256(csv_raw).hexdigest(),
 'csv_size_bytes':len(csv_raw),
 'csv_content_type':csv_ctype,
 'selected_year':year or None,
 'selected_month':month_name,
 'available_csv_count':len(links),
 'retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),
 'license':'FDA public data / US federal public information',
 'scope':'bounded Purple Book biologic product identifier/relationship metadata only; strength values, label text, prescribing/safety guidance, patent details, clinical-use interpretation, and graph/mirror wiring excluded'
}
(dest/'source-receipt.json').write_text(json.dumps(receipt,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
print(f'updated Purple Book: {receipt["version"]} url={csv_url} bytes={len(csv_raw)} sha={receipt["csv_sha256"]}')
PY
