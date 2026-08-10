#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="${ECS_SCHEMA_DEST:-$ROOT/ingest/telemetry/elastic-ecs-schema}"
REF="${ECS_SCHEMA_REF:-v9.4.0}"
TMP="${TMPDIR:-/tmp}/pnix-ecs-schema-$$"
rm -rf "$TMP"
mkdir -p "$TMP" "$DEST/raw"
URL="https://github.com/elastic/ecs/archive/refs/tags/$REF.tar.gz"
curl -fsSL "$URL" -o "$TMP/ecs.tar.gz"
tar -xzf "$TMP/ecs.tar.gz" -C "$TMP"
SRC="$TMP/ecs-${REF#v}"
python3 - "$DEST" "$REF" "$URL" "$SRC" <<'PY'
import hashlib,json,os,sys,time
out,ref,url,src=sys.argv[1:]
rawdir=os.path.join(out,'raw')
os.makedirs(rawdir,exist_ok=True)
records=[]
schema_dir=os.path.join(src,'schemas')
for fn in sorted(os.listdir(schema_dir)):
    if not fn.endswith(('.yml','.yaml')) or fn.lower().startswith('readme'):
        continue
    p=os.path.join(schema_dir,fn)
    b=open(p,'rb').read()
    open(os.path.join(rawdir,fn),'wb').write(b)
    records.append({'source_path':'schemas/'+fn,'local_file':fn,'url':f'https://github.com/elastic/ecs/blob/{ref}/schemas/{fn}','sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b),'role':'schema_yaml'})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'Elastic Common Schema official schema YAML','ref':ref,'retrieved_at_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'source_urls':['https://github.com/elastic/ecs','https://github.com/elastic/ecs/tree/'+ref+'/schemas',url],'license':'Apache-2.0','scope':'schema YAML structural fields only; no prose/examples/real logs/PII/detections/execution/graph wiring','files':records,'file_count':len(records)}
with open(os.path.join(out,'source-receipt.json'),'w',encoding='utf-8') as fp:
    json.dump(receipt,fp,ensure_ascii=False,indent=2,sort_keys=True); fp.write('\n')
print(f'downloaded Elastic ECS schemas: ref={ref} yaml_files={len(records)} bytes={sum(r["bytes"] for r in records)} -> {out}')
PY
rm -rf "$TMP"
