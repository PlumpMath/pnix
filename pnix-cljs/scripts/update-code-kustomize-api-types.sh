#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="${KUSTOMIZE_API_TYPES_DEST:-$ROOT/ingest/code/kustomize-api-types}"
REF="${KUSTOMIZE_API_TYPES_REF:-kustomize/v5.8.1}"
TMP="${TMPDIR:-/tmp}/pnix-kustomize-api-types-$$"
rm -rf "$TMP"; mkdir -p "$TMP" "$DEST/raw"
URL="https://github.com/kubernetes-sigs/kustomize/archive/refs/tags/$REF.tar.gz"
curl -fsSL "$URL" -o "$TMP/kustomize.tar.gz"
tar -xzf "$TMP/kustomize.tar.gz" -C "$TMP"
SRC="$(find "$TMP" -maxdepth 1 -type d -name 'kustomize-*' | head -1)"
python3 - "$DEST" "$REF" "$URL" "$SRC" <<'PY'
import hashlib,json,os,sys,time
out,ref,url,src=sys.argv[1:]
rawdir=os.path.join(out,'raw'); os.makedirs(rawdir,exist_ok=True)
records=[]
d=os.path.join(src,'api/types')
for fn in sorted(os.listdir(d)):
    if not fn.endswith('.go') or fn.endswith('_test.go'):
        continue
    rel='api/types/'+fn; p=os.path.join(src,rel); b=open(p,'rb').read(); local=rel.replace('/','__')
    open(os.path.join(rawdir,local),'wb').write(b)
    records.append({'source_path':rel,'local_file':local,'url':f'https://github.com/kubernetes-sigs/kustomize/blob/{ref}/{rel}','sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b),'role':'go_model_source'})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'Kustomize API types structural metadata','ref':ref,'retrieved_at_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'source_urls':['https://github.com/kubernetes-sigs/kustomize','https://github.com/kubernetes-sigs/kustomize/tree/'+ref+'/api/types',url],'license':'Apache-2.0','scope':'Go model struct/tag identifiers only; no source bodies/tests/real overlays/patches/manifests/cluster secrets/execution/graph wiring','files':records}
with open(os.path.join(out,'source-receipt.json'),'w',encoding='utf-8') as fp: json.dump(receipt,fp,ensure_ascii=False,indent=2,sort_keys=True); fp.write('\n')
print(f'downloaded Kustomize API types: ref={ref} files={len(records)} bytes={sum(r["bytes"] for r in records)} -> {out}')
PY
rm -rf "$TMP"
