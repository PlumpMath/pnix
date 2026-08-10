#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="${OPENMETRICS_DEST:-$ROOT/ingest/telemetry/openmetrics}"
REF="${OPENMETRICS_REF:-v1.0.0}"
mkdir -p "$DEST/raw"
base="https://raw.githubusercontent.com/prometheus/OpenMetrics/$REF"
files=(proto/openmetrics_data_model.proto specification/OpenMetrics.md)
for f in "${files[@]}"; do mkdir -p "$DEST/raw/$(dirname "$f")"; curl -fsSL "$base/$f" -o "$DEST/raw/$f"; done
python3 - "$DEST" "$REF" "$base" "${files[@]}" <<'PY'
import hashlib,json,os,sys,time
out,ref,base,*files=sys.argv[1:]
records=[]
for f in files:
    p=os.path.join(out,'raw',f); b=open(p,'rb').read()
    records.append({'file':f,'url':f'{base}/{f}','sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b),'role':'proto' if f.endswith('.proto') else 'spec_markdown'})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'OpenMetrics official specification structural metadata','ref':ref,'retrieved_at_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'source_urls':['https://github.com/prometheus/OpenMetrics','https://github.com/prometheus/OpenMetrics/tree/'+ref],'license':'Apache-2.0','scope':'proto and spec structural rows only; no prose bodies/real metrics/scrape targets/alert routing/credentials/execution/graph wiring','files':records}
with open(os.path.join(out,'source-receipt.json'),'w',encoding='utf-8') as fp: json.dump(receipt,fp,ensure_ascii=False,indent=2,sort_keys=True); fp.write('\n')
print(f'downloaded OpenMetrics: ref={ref} files={len(records)} bytes={sum(r["bytes"] for r in records)} -> {out}')
PY
