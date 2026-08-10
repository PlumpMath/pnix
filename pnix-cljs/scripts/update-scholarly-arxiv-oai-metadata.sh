#!/usr/bin/env bash
# arXiv OAI-PMH metadata snapshot.
# Respects legacy API rate limit: one request every >=3 seconds, single connection.
# No PDFs, TeX/source files, fulltext, abstract/title/author values, bulk S3 payloads, or graph wiring.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${ARXIV_OAI_DEST:-$ROOT/ingest/scholarly/arxiv-oai-metadata}"
BASE="${ARXIV_OAI_BASE:-https://export.arxiv.org/oai2}"
FROM="${ARXIV_OAI_FROM:-2026-06-01}"
UNTIL="${ARXIV_OAI_UNTIL:-$FROM}"
METADATA_PREFIX="${ARXIV_OAI_METADATA_PREFIX:-arXiv}"
LIMIT="${ARXIV_OAI_LIMIT:-100}"
rm -rf "$DEST"
mkdir -p "$DEST/raw"
python3 - "$DEST" "$BASE" "$FROM" "$UNTIL" "$METADATA_PREFIX" "$LIMIT" <<'PY'
import datetime as dt, hashlib, json, pathlib, sys, time, urllib.parse, urllib.request
out=pathlib.Path(sys.argv[1]); base=sys.argv[2]; from_date=sys.argv[3]; until_date=sys.argv[4]; prefix=sys.argv[5]; limit=int(sys.argv[6])
requests=[('Identify',{'verb':'Identify'}),('ListMetadataFormats',{'verb':'ListMetadataFormats'}),('ListSets',{'verb':'ListSets'}),('ListRecords',{'verb':'ListRecords','metadataPrefix':prefix,'from':from_date,'until':until_date})]
files=[]; request_rows=[]
headers={'User-Agent':'pnix-ingest-arxiv-oai/0.1 (metadata-only; no fulltext)'}
for i,(name,params) in enumerate(requests):
    if i: time.sleep(3.2)
    url=base+'?'+urllib.parse.urlencode(params)
    req=urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(req, timeout=120) as r:
        raw=r.read()
        status=r.status
        content_type=r.headers.get('content-type')
    rel=pathlib.Path('raw')/(name + '.xml')
    (out/rel).write_bytes(raw)
    files.append({'source_path':str(rel),'relative_path':str(rel),'url':url,'sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw),'role':'arxiv_oai_xml','verb':name})
    request_rows.append({'verb':name,'url':url,'http_status':status,'content_type':content_type,'size_bytes':len(raw)})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'arXiv OAI-PMH descriptive metadata','retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'source_urls':['https://info.arxiv.org/help/oa/index.html','https://info.arxiv.org/help/oa/metadataPolicy.html','https://info.arxiv.org/help/api/tou.html','https://info.arxiv.org/help/bulk_data.html',base],'license':'CC0-1.0 for descriptive metadata; e-print payloads excluded','scope':'official OAI-PMH capability/set/format metadata and bounded identifier/category/license/date slice only; no PDF/TeX/fulltext, abstract/title/author/comment/journal-ref values, bulk S3 payloads, linked payloads, profiling/ranking, or graph wiring','base_url':base,'from':from_date,'until':until_date,'metadata_prefix':prefix,'record_limit':limit,'rate_limit_seconds_between_requests':3.2,'files':files,'requests':request_rows,'fulltext_payloads_downloaded':False}
(out/'source-receipt.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True)+'\n', encoding='utf-8')
print(f'downloaded arXiv OAI metadata snapshot: from={from_date} until={until_date} prefix={prefix} -> {out}')
PY
