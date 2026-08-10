#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/maritime/usace-waterborne-commerce-catalog"
mkdir -p "$OUT"
UA="pnix-ingest/1.0 (catalog metadata only; contact: local)"
SEARCH_URL="${USACE_WCSC_SEARCH_URL:-https://usace.contentdm.oclc.org/digital/api/search/collection/p16021coll2/searchterm/Waterborne%20Commerce/field/title/maxRecords/50}"
SAMPLE_PAGE_URL="${USACE_WCSC_SAMPLE_PAGE_URL:-https://usace.contentdm.oclc.org/digital/collection/p16021coll2/id/1473/}"
curl -fsSL -A "$UA" "$SEARCH_URL" -o "$OUT/search.json"
curl -fsSL -A "$UA" "$SAMPLE_PAGE_URL" -o "$OUT/sample-page.html"
python3 - "$OUT" "$SEARCH_URL" "$SAMPLE_PAGE_URL" <<'PY'
import hashlib,json,pathlib,sys,datetime
out=pathlib.Path(sys.argv[1]); urls=sys.argv[2:]
files=[]
for p in sorted(out.iterdir()):
    if p.name=='source-manifest.json' or not p.is_file(): continue
    files.append({'path':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
manifest={'schema':'pnix.source_manifest.v1','source_id':'usace-waterborne-commerce-catalog','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':urls,'files':files,'policy':'USACE CONTENTdm catalog metadata only; report/statistics payloads and operational/security guidance excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'files':len(files),'out':str(out)},ensure_ascii=False))
PY
