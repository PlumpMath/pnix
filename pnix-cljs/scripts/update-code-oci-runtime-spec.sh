#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="${OCI_RUNTIME_SPEC_DEST:-$ROOT/ingest/code/oci-runtime-spec}"
REF="${OCI_RUNTIME_SPEC_REF:-v1.3.0}"
RAW="$DEST/raw"
mkdir -p "$RAW"
files=(config-schema.json config-linux.json config-windows.json config-solaris.json config-freebsd.json config-vm.json config-zos.json defs.json defs-linux.json defs-windows.json defs-freebsd.json defs-vm.json defs-zos.json features-schema.json features-linux.json state-schema.json)
base="https://raw.githubusercontent.com/opencontainers/runtime-spec/$REF/schema"
for f in "${files[@]}"; do curl -fsSL "$base/$f" -o "$RAW/$f"; done
python3 - "$DEST" "$REF" "$base" "${files[@]}" <<'PY'
import hashlib,json,os,sys,time
out,ref,base,*files=sys.argv[1:]
raw=os.path.join(out,'raw'); records=[]
for f in files:
    b=open(os.path.join(raw,f),'rb').read()
    records.append({'file':f,'url':f'{base}/{f}','sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b),'role':'json_schema'})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'OCI Runtime Specification official JSON schemas','ref':ref,'retrieved_at_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'source_urls':['https://github.com/opencontainers/runtime-spec','https://github.com/opencontainers/runtime-spec/tree/'+ref+'/schema'],'license':'Apache-2.0','scope':'official schema JSON only; no real runtime configs/state/host paths/devices/env/hooks/execution/mutation/graph wiring','files':records}
os.makedirs(out,exist_ok=True)
with open(os.path.join(out,'source-receipt.json'),'w',encoding='utf-8') as fp: json.dump(receipt,fp,ensure_ascii=False,indent=2,sort_keys=True); fp.write('\n')
print(f'downloaded OCI runtime-spec schemas: ref={ref} files={len(records)} bytes={sum(r["bytes"] for r in records)} -> {out}')
PY
