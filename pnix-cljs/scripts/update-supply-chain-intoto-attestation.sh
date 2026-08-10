#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="${INTOTO_ATTESTATION_DEST:-$ROOT/ingest/supply-chain/intoto-attestation}"
REF="${INTOTO_ATTESTATION_REF:-v1.2.0}"
TMP="${TMPDIR:-/tmp}/pnix-intoto-attestation-$$"
rm -rf "$TMP"; mkdir -p "$TMP" "$DEST/raw"
URL="https://github.com/in-toto/attestation/archive/refs/tags/$REF.tar.gz"
curl -fsSL "$URL" -o "$TMP/attestation.tar.gz"
tar -xzf "$TMP/attestation.tar.gz" -C "$TMP"
SRC="$TMP/attestation-${REF#v}"
python3 - "$DEST" "$REF" "$URL" "$SRC" <<'PY'
import hashlib,json,os,sys,time
out,ref,url,src=sys.argv[1:]
rawdir=os.path.join(out,'raw'); os.makedirs(rawdir,exist_ok=True)
records=[]
for root,_,files in os.walk(src):
    for fn in files:
        rel=os.path.relpath(os.path.join(root,fn),src)
        if not (rel.endswith('.proto') or rel.startswith('spec/predicates/') and rel.endswith('.md') or rel.startswith('spec/v1/') and rel.endswith('.md')):
            continue
        b=open(os.path.join(src,rel),'rb').read(); local=rel.replace('/','__')
        open(os.path.join(rawdir,local),'wb').write(b)
        records.append({'source_path':rel,'local_file':local,'url':f'https://github.com/in-toto/attestation/blob/{ref}/{rel}','sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b),'role':'proto_schema' if rel.endswith('.proto') else 'markdown_spec'})
records.sort(key=lambda r:r['source_path'])
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'in-toto Attestation official schema metadata','ref':ref,'retrieved_at_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'source_urls':['https://github.com/in-toto/attestation','https://github.com/in-toto/attestation/tree/'+ref,url],'license':'Apache-2.0','scope':'protobuf/spec structural identifiers only; no prose/examples/real attestations/signatures/keys/logs/secrets/execution/graph wiring','files':records}
with open(os.path.join(out,'source-receipt.json'),'w',encoding='utf-8') as fp: json.dump(receipt,fp,ensure_ascii=False,indent=2,sort_keys=True); fp.write('\n')
print(f'downloaded in-toto attestation: ref={ref} files={len(records)} bytes={sum(r["bytes"] for r in records)} -> {out}')
PY
rm -rf "$TMP"
