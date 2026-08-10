#!/usr/bin/env bash
# eCFR 49 CFR 172.101 updater. Downloads section XML; generator stores identifier rows only.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/regulatory/ecfr-49-172-101-hazmat-identifiers"
mkdir -p "$DEST"
python3 - "$DEST/title-49-172-101.xml" "$DEST/source-receipt.json" <<'PY'
import datetime, hashlib, json, pathlib, sys, urllib.request
xml_path, receipt_path = map(pathlib.Path, sys.argv[1:])
ua={'User-Agent':'pnix-ingest/1.0 (hazmat identifier metadata only)'}
def fetch_json(url):
    with urllib.request.urlopen(urllib.request.Request(url,headers=ua),timeout=60) as r:
        return json.load(r), r.geturl(), r.headers.get('content-type','')
def fetch(url):
    with urllib.request.urlopen(urllib.request.Request(url,headers=ua),timeout=120) as r:
        return r.read(), r.geturl(), r.headers.get('content-type','')
titles, titles_final, titles_ctype = fetch_json('https://www.ecfr.gov/api/versioner/v1/titles.json')
t49=next(t for t in titles.get('titles',[]) if str(t.get('number'))=='49')
date=t49.get('up_to_date_as_of') or t49.get('latest_issue_date') or t49.get('latest_amended_on')
url=f'https://www.ecfr.gov/api/versioner/v1/full/{date}/title-49.xml?part=172&section=172.101'
raw, final, ctype = fetch(url)
xml_path.write_bytes(raw)
sha=hashlib.sha256(raw).hexdigest()
receipt={
  'schema':'pnix.ingest.source_receipt.v1',
  'source':'eCFR 49 CFR 172.101 Hazardous Materials Table',
  'version':f'title-49-up-to-date-as-of-{date}',
  'title_49_dates':t49,
  'url':url,
  'final_url':final,
  'sha256':sha,
  'size_bytes':len(raw),
  'content_type':ctype,
  'retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),
  'license':'US Government public domain / eCFR XML no downstream copyright restriction',
  'scope':'49 CFR 172.101 XML downloaded; generator stores only Hazardous Materials Table UN/NA identifier metadata and excludes packaging/quantity/stowage/special provisions/RQ/emergency/compliance advice'
}
receipt_path.write_text(json.dumps(receipt,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
print(f'updated eCFR hazmat identifiers source: date={date} sha={sha} bytes={len(raw)}')
PY
