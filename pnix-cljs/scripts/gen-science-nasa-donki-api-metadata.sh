#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/ingest/science/nasa-donki-api-metadata/raw"
OUT="$ROOT/stdlib/lib/corpus/nasa-donki-api-metadata.generated.px"
RECEIPT="$ROOT/ingest/science/nasa-donki-api-metadata/source-receipt.json"
if [[ ! -d "$SRC" ]]; then
  echo "missing $SRC; run scripts/update-science-nasa-donki-api-metadata.sh first" >&2
  exit 1
fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import html, json, re, sys, urllib.parse
from pathlib import Path
src=Path(sys.argv[1]); out=Path(sys.argv[2]); receipt_path=Path(sys.argv[3])
receipt=json.loads(receipt_path.read_text()) if receipt_path.exists() else {}
def esc(s): return json.dumps(str(s), ensure_ascii=False)
def to_pnix(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,list): return '[ ' + ' '.join(to_pnix(x) for x in v) + ' ]'
    if isinstance(v,dict): return '{ ' + ' '.join(f'{k} = {to_pnix(val)};' for k,val in v.items() if val is not None) + ' }'
    return esc(v)
files=[]; seen={}; endpoints=[]; params={}; catalogs=set()
for path in sorted(src.glob('*.html')):
    text=path.read_text(errors='replace')
    files.append({'file':path.name,'bytes':path.stat().st_size,'lines':len(text.splitlines())})
    text=html.unescape(text).replace('\\u0026','&')
    for m in re.finditer(r'https?://[^\s"\'<>]+/DONKI/WS/get/[^\s"\'<>]+', text):
        raw=m.group(0).rstrip(').,;')
        parsed=urllib.parse.urlparse(raw)
        endpoint=parsed.path
        query=urllib.parse.parse_qsl(parsed.query, keep_blank_values=True)
        key=(endpoint, tuple((k,v) for k,v in query))
        if key in seen: continue
        seen[key]=True
        code=endpoint.split('/')[-1].replace('.txt','')
        catalogs.add(code)
        sample=not any(v in ('yyyy-MM-dd','LOCATION','CATALOG','ALL','all','') for _,v in query)
        endpoints.append({'source_file':path.name,'base_url':f'{parsed.scheme}://{parsed.netloc}','path':endpoint,'event_code':code,'format':'txt' if endpoint.endswith('.txt') else 'json','is_sample':sample,'query': [{'name':k,'value':v} for k,v in query]})
        for k,v in query:
            p=params.setdefault((code,k), {'event_code':code,'name':k,'sample_values':[]})
            if v and len(p['sample_values'])<8 and v not in p['sample_values']:
                p['sample_values'].append(v)
# Also preserve API portal top-level DONKI mention without endpoint calls.
data={
 'schema':'space_weather.nasa_donki.api_metadata.v1',
 'source':'NASA CCMC DONKI public documentation endpoint metadata',
 'license':'USGOV-PUBLIC-METADATA',
 'scope':'endpoint path/query metadata only; no event data fetched',
 'receipt_sources':receipt.get('sources',[]),
 'source_files':files,
 'catalogs':[{'event_code':c} for c in sorted(catalogs)],
 'endpoints':endpoints,
 'parameters':list(params.values()),
 'exclusions':['event JSON','forecasts','notifications payloads','model outputs','API keys','logos','prose bodies','execution','mirror/graph wiring']
}
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(to_pnix(data)+'\n')
print(f"generated {out}: files={len(files)} endpoints={len(endpoints)} catalogs={len(catalogs)} params={len(params)} bytes={out.stat().st_size}")
PY
