#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="${SIGSTORE_PROTOBUF_DEST:-$ROOT/ingest/supply-chain/sigstore-protobuf-specs}"
REF="${SIGSTORE_PROTOBUF_REF:-v0.5.1}"
TMP="${TMPDIR:-/tmp}/pnix-sigstore-protobuf-$$"
rm -rf "$TMP"; mkdir -p "$TMP" "$DEST/raw"
URL="https://github.com/sigstore/protobuf-specs/archive/refs/tags/$REF.tar.gz"
curl -fsSL "$URL" -o "$TMP/sigstore-protobuf.tar.gz"
tar -xzf "$TMP/sigstore-protobuf.tar.gz" -C "$TMP"
SRC="$TMP/protobuf-specs-${REF#v}"
python3 - "$DEST" "$REF" "$URL" "$SRC" <<'PY'
import hashlib,json,os,sys,time
out,ref,url,src=sys.argv[1:]
rawdir=os.path.join(out,'raw'); os.makedirs(rawdir,exist_ok=True)
records=[]
for fn in sorted(os.listdir(os.path.join(src,'protos'))):
    if not fn.endswith('.proto'): continue
    rel='protos/'+fn; p=os.path.join(src,rel); b=open(p,'rb').read()
    open(os.path.join(rawdir,fn),'wb').write(b)
    records.append({'source_path':rel,'local_file':fn,'url':f'https://github.com/sigstore/protobuf-specs/blob/{ref}/{rel}','sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b),'role':'proto_schema'})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'Sigstore protobuf specifications','ref':ref,'retrieved_at_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'source_urls':['https://github.com/sigstore/protobuf-specs','https://github.com/sigstore/protobuf-specs/tree/'+ref,url],'license':'Apache-2.0','scope':'proto structure only; no actual signatures/keys/certs/rekor entries/bundles/verification/live services/execution/graph wiring','files':records}
with open(os.path.join(out,'source-receipt.json'),'w',encoding='utf-8') as fp: json.dump(receipt,fp,ensure_ascii=False,indent=2,sort_keys=True); fp.write('\n')
print(f'downloaded Sigstore protobuf specs: ref={ref} proto_files={len(records)} bytes={sum(r["bytes"] for r in records)} -> {out}')
PY
rm -rf "$TMP"
