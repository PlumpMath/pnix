#!/usr/bin/env bash
# OpenAlex API endpoint count/schema-key manifest snapshot.
# Fetches one item only to discover field keys, then discards result values in generator.
# No work/title/abstract/person values, abstract_inverted_index, citation edges, snapshot payloads, or graph wiring.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${OPENALEX_DEST:-$ROOT/ingest/scholarly/openalex-api-manifest}"
MAILTO="${OPENALEX_MAILTO:-metadata-only@example.invalid}"
ENDPOINTS="${OPENALEX_ENDPOINTS:-works,authors,sources,institutions,topics,publishers,funders}"
rm -rf "$DEST"
mkdir -p "$DEST/raw"
python3 - "$DEST" "$MAILTO" "$ENDPOINTS" <<'PY'
import datetime as dt, hashlib, json, pathlib, sys, time, urllib.parse, urllib.request
out=pathlib.Path(sys.argv[1]); mailto=sys.argv[2]; endpoints=[x.strip() for x in sys.argv[3].split(',') if x.strip()]
base='https://api.openalex.org'
rows=[]; files=[]
headers={'User-Agent':f'pnix-ingest-openalex-api-manifest/0.1 (mailto:{mailto})'}
for i,ep in enumerate(endpoints):
    url=f'{base}/{ep}?'+urllib.parse.urlencode({'per-page':'1','page':'1','mailto':mailto})
    req=urllib.request.Request(url, headers=headers)
    with urllib.request.urlopen(req, timeout=60) as r:
        raw=r.read(); status=r.status; content_type=r.headers.get('content-type')
    rel=pathlib.Path('raw')/(ep+'.sample1.json')
    (out/rel).write_bytes(raw)
    data=json.loads(raw.decode('utf-8'))
    meta=data.get('meta') or {}; results=data.get('results') or []
    rows.append({'endpoint':ep,'url':url,'http_status':status,'content_type':content_type,'api_count':meta.get('count'),'db_response_time_ms':meta.get('db_response_time_ms'),'page':meta.get('page'),'per_page':meta.get('per_page'),'result_count_downloaded':len(results),'result_keys':sorted((results[0] or {}).keys()) if results else []})
    files.append({'source_path':str(rel),'relative_path':str(rel),'url':url,'sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw),'role':'openalex_api_sample1_json','endpoint':ep})
    time.sleep(0.15)
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'OpenAlex API endpoint count/schema-key manifest','retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'source_urls':['https://developers.openalex.org/','https://github.com/ourresearch/openalex-docs/blob/main/license.md','https://developers.openalex.org/download/download-to-machine','https://developers.openalex.org/api-reference/works/get-a-single-work',base],'license':'CC0-1.0; endpoint count/schema-key manifest only','scope':'official OpenAlex API endpoint counts and field-key inventory only; no API result item values, titles, abstracts, abstract_inverted_index, person values, citation edges, snapshot payloads, linked payloads, profiling/ranking, or graph wiring','mailto_used':mailto,'base_url':base,'endpoint_rows':rows,'files':files,'record_payload_values_downloaded_for_key_discovery_but_not_ingested':True,'snapshot_payloads_downloaded':False}
(out/'source-receipt.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True)+'\n', encoding='utf-8')
print(f'downloaded OpenAlex API manifest: endpoints={len(rows)} -> {out}')
PY
