#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="${OTEL_SEMCONV_DEST:-$ROOT/ingest/telemetry/opentelemetry-semconv}"
REF="${OTEL_SEMCONV_REF:-v1.42.0}"
mkdir -p "$DEST/raw"
python3 - "$DEST" "$REF" <<'PY'
import hashlib,json,os,sys,time,urllib.request
out,ref=sys.argv[1:]
repo='open-telemetry/semantic-conventions'
ua={'User-Agent':'pnix-otel-semconv-ingest/1.0','Accept':'application/vnd.github+json'}
def get_json(url):
    req=urllib.request.Request(url,headers=ua)
    with urllib.request.urlopen(req,timeout=40) as r: return json.load(r)
tree=get_json(f'https://api.github.com/repos/{repo}/git/trees/{ref}?recursive=1')['tree']
paths=[x['path'] for x in tree if x.get('type')=='blob' and x['path'].startswith('model/') and x['path'].endswith(('.yaml','.yml'))]
records=[]; rawdir=os.path.join(out,'raw')
os.makedirs(rawdir,exist_ok=True)
for path in paths:
    url=f'https://raw.githubusercontent.com/{repo}/{ref}/{path}'
    req=urllib.request.Request(url,headers={'User-Agent':'pnix-otel-semconv-ingest/1.0'})
    with urllib.request.urlopen(req,timeout=40) as r: b=r.read()
    rel=path.replace('/','__')
    open(os.path.join(rawdir,rel),'wb').write(b)
    records.append({'source_path':path,'local_file':rel,'url':url,'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b),'role':'model_yaml'})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'OpenTelemetry Semantic Conventions model YAML','ref':ref,'retrieved_at_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'source_urls':['https://github.com/open-telemetry/semantic-conventions','https://github.com/open-telemetry/semantic-conventions/tree/'+ref+'/model'],'license':'Apache-2.0','scope':'model YAML structure only; no prose/real telemetry/credentials/alert routing/execution/graph wiring','files':records,'file_count':len(records)}
with open(os.path.join(out,'source-receipt.json'),'w',encoding='utf-8') as fp: json.dump(receipt,fp,ensure_ascii=False,indent=2,sort_keys=True); fp.write('\n')
print(f'downloaded OpenTelemetry semconv: ref={ref} yaml_files={len(records)} bytes={sum(r["bytes"] for r in records)} -> {out}')
PY
