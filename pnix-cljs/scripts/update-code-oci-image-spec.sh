#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="${OCI_IMAGE_SPEC_DEST:-$ROOT/ingest/code/oci-image-spec}"
REF="${OCI_IMAGE_SPEC_REF:-v1.1.1}"
RAW="$DEST/raw"
mkdir -p "$RAW"
files=(config-schema.json content-descriptor.json defs-descriptor.json defs.json image-index-schema.json image-layout-schema.json image-manifest-schema.json)
base="https://raw.githubusercontent.com/opencontainers/image-spec/$REF/schema"
for f in "${files[@]}"; do
  curl -fsSL "$base/$f" -o "$RAW/$f"
done
python3 - "$DEST" "$REF" "$base" "${files[@]}" <<'PY'
import hashlib,json,os,sys,time
out,ref,base,*files=sys.argv[1:]
raw=os.path.join(out,'raw')
records=[]
for f in files:
    p=os.path.join(raw,f); b=open(p,'rb').read()
    records.append({'file':f,'url':f'{base}/{f}','sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b),'role':'json_schema'})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'OCI Image Specification official JSON schemas','ref':ref,'retrieved_at_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'source_urls':['https://github.com/opencontainers/image-spec','https://github.com/opencontainers/image-spec/tree/'+ref+'/schema'],'license':'Apache-2.0','scope':'official schema JSON only; no image blobs/registry payloads/credentials/signatures/source tests/execution/graph wiring','files':records}
os.makedirs(out,exist_ok=True)
with open(os.path.join(out,'source-receipt.json'),'w',encoding='utf-8') as fp:
    json.dump(receipt,fp,ensure_ascii=False,indent=2,sort_keys=True); fp.write('\n')
print(f'downloaded OCI image-spec schemas: ref={ref} files={len(records)} bytes={sum(r["bytes"] for r in records)} -> {out}')
PY
