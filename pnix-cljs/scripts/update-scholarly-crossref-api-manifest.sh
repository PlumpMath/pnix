#!/usr/bin/env bash
# Crossref REST API endpoint manifest snapshot.
# Uses rows=0 calls only. No DOI records, person/title/abstract/reference values, snapshots, public data file payloads, or graph wiring.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${CROSSREF_DEST:-$ROOT/ingest/scholarly/crossref-api-manifest}"
MAILTO="${CROSSREF_MAILTO:-metadata-only@example.invalid}"
rm -rf "$DEST"
mkdir -p "$DEST/raw"
python3 - "$DEST" "$MAILTO" <<'PY'
import datetime as dt, hashlib, json, pathlib, sys, urllib.parse, urllib.request
out=pathlib.Path(sys.argv[1]); mailto=sys.argv[2]
base='https://api.crossref.org'
endpoints=['works','types','members','funders','licenses','journals']
rows=[]; files=[]
headers={'User-Agent':f'pnix-ingest-crossref-manifest/0.1 (mailto:{mailto})'}
for ep in endpoints:
    qs=urllib.parse.urlencode({'rows':'0','mailto':mailto})
    url=f'{base}/{ep}?{qs}'
    req=urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(req, timeout=60) as r:
        raw=r.read()
        status_code=r.status
        content_type=r.headers.get('content-type')
    data=json.loads(raw.decode('utf-8'))
    rel=pathlib.Path('raw')/(ep + '.rows0.json')
    (out/rel).write_bytes(raw)
    msg=data.get('message') or {}
    rows.append({'endpoint':ep,'url':url,'http_status':status_code,'content_type':content_type,'api_status':data.get('status'),'total_results':msg.get('total-results'),'items_per_page':msg.get('items-per-page'),'items_count':len(msg.get('items') or []),'query':msg.get('query')})
    files.append({'source_path':str(rel),'relative_path':str(rel),'url':url,'sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw),'role':'crossref_rows0_endpoint_json','endpoint':ep})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'Crossref REST API endpoint manifest','retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'source_urls':['https://www.crossref.org/documentation/retrieve-metadata/rest-api/','https://github.com/Crossref/rest-api-doc','https://www.crossref.org/documentation/retrieve-metadata/bulk-downloads/','https://api.crossref.org/'],'license':'Crossref Free Services metadata rights / no ownership claims; endpoint manifest rows=0 only','scope':'official REST API endpoint manifest and rows=0 counts only; no DOI record payloads, titles, abstracts, authors, references, work license arrays, public data file payloads, snapshots, linked payloads, profiling/ranking outputs, or graph wiring','mailto_used':mailto,'base_url':base,'endpoint_rows':rows,'files':files,'rows_zero_only':True,'doi_record_payloads_ingested':False}
(out/'source-receipt.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True)+'\n', encoding='utf-8')
print(f'downloaded Crossref API manifest: endpoints={len(rows)} -> {out}')
PY
