#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="${PROM_DATA_MODEL_DEST:-$ROOT/ingest/telemetry/prometheus-data-model}"
REF="${PROM_DATA_MODEL_REF:-v3.12.0}"
mkdir -p "$DEST/raw"
base="https://raw.githubusercontent.com/prometheus/prometheus/$REF"
files=(model/metadata/metadata.go model/value/value.go model/labels/labels_common.go docs/querying/basics.md)
for f in "${files[@]}"; do mkdir -p "$DEST/raw/$(dirname "$f")"; curl -fsSL "$base/$f" -o "$DEST/raw/$f"; done
python3 - "$DEST" "$REF" "$base" "${files[@]}" <<'PY'
import hashlib,json,os,sys,time
out,ref,base,*files=sys.argv[1:]
records=[]
for f in files:
    b=open(os.path.join(out,'raw',f),'rb').read()
    records.append({'file':f,'url':f'{base}/{f}','sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b),'role':'go_source' if f.endswith('.go') else 'markdown_doc'})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'Prometheus data model structural metadata','ref':ref,'retrieved_at_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'source_urls':['https://github.com/prometheus/prometheus','https://github.com/prometheus/prometheus/tree/'+ref],'license':'Apache-2.0','scope':'structural identifiers only; no Go/prose bodies, real metrics, scrape targets/logs/credentials/alert routing/execution/graph wiring','files':records}
with open(os.path.join(out,'source-receipt.json'),'w',encoding='utf-8') as fp: json.dump(receipt,fp,ensure_ascii=False,indent=2,sort_keys=True); fp.write('\n')
print(f'downloaded Prometheus data model snapshot: ref={ref} files={len(records)} bytes={sum(r["bytes"] for r in records)} -> {out}')
PY
