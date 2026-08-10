#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="${SLSA_SPEC_DEST:-$ROOT/ingest/supply-chain/slsa-spec}"
REF="${SLSA_SPEC_REF:-main}"
TMP="${TMPDIR:-/tmp}/pnix-slsa-spec-$$"
rm -rf "$TMP"; mkdir -p "$TMP" "$DEST/raw"
if [ "$REF" = "main" ]; then URL="https://github.com/slsa-framework/slsa/archive/refs/heads/main.tar.gz"; DIR="slsa-main"; else URL="https://github.com/slsa-framework/slsa/archive/refs/tags/$REF.tar.gz"; DIR="slsa-${REF#v}"; fi
curl -fsSL "$URL" -o "$TMP/slsa.tar.gz"
tar -xzf "$TMP/slsa.tar.gz" -C "$TMP"
SRC="$TMP/$DIR"
python3 - "$DEST" "$REF" "$URL" "$SRC" <<'PY'
import hashlib,json,os,shutil,sys,time
out,ref,url,src=sys.argv[1:]
selected=['spec/schema/provenance.cue','spec/schema/provenance.proto','spec/provenance.md','spec/build-requirements.md','spec/source-requirements.md','spec/requirements.md','spec/verified-properties.md','spec/verification_summary.md','spec/tracks.md']
rawdir=os.path.join(out,'raw'); os.makedirs(rawdir,exist_ok=True)
records=[]
for rel in selected:
    p=os.path.join(src,rel)
    if not os.path.exists(p): continue
    b=open(p,'rb').read(); local=rel.replace('/','__')
    open(os.path.join(rawdir,local),'wb').write(b)
    records.append({'source_path':rel,'local_file':local,'url':f'https://github.com/slsa-framework/slsa/blob/{ref}/{rel}','sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b),'role':'schema' if rel.endswith(('.cue','.proto')) else 'markdown_spec'})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'SLSA specification structural metadata','ref':ref,'retrieved_at_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'source_urls':['https://github.com/slsa-framework/slsa','https://slsa.dev/spec/',url],'license':'CC-BY-4.0 / repository permissive source','scope':'schema and token structure only; no prose bodies/examples/real attestations/build logs/secrets/execution/graph wiring','files':records}
with open(os.path.join(out,'source-receipt.json'),'w',encoding='utf-8') as fp: json.dump(receipt,fp,ensure_ascii=False,indent=2,sort_keys=True); fp.write('\n')
print(f'downloaded SLSA spec snapshot: ref={ref} files={len(records)} bytes={sum(r["bytes"] for r in records)} -> {out}')
PY
rm -rf "$TMP"
