#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="${OCSF_SCHEMA_DEST:-$ROOT/ingest/telemetry/ocsf-schema}"
REF="${OCSF_SCHEMA_REF:-1.8.0}"
TMP="${TMPDIR:-/tmp}/pnix-ocsf-schema-$$"
rm -rf "$TMP"
mkdir -p "$TMP" "$DEST/raw"
URL="https://github.com/ocsf/ocsf-schema/archive/refs/tags/$REF.tar.gz"
curl -fsSL "$URL" -o "$TMP/ocsf-schema.tar.gz"
tar -xzf "$TMP/ocsf-schema.tar.gz" -C "$TMP"
SRC="$TMP/ocsf-schema-$REF"
python3 - "$DEST" "$REF" "$URL" "$SRC" <<'PY'
import hashlib,json,os,shutil,sys,time
out,ref,url,src=sys.argv[1:]
roots=('dictionary.json','categories/','objects/','events/','profiles/','extensions/')
rawdir=os.path.join(out,'raw')
os.makedirs(rawdir,exist_ok=True)
records=[]
for root,_,files in os.walk(src):
    for fn in files:
        rel=os.path.relpath(os.path.join(root,fn),src)
        if not rel.endswith('.json'): continue
        if not (rel=='dictionary.json' or rel.startswith(roots[1:])): continue
        b=open(os.path.join(src,rel),'rb').read()
        local=rel.replace('/','__')
        open(os.path.join(rawdir,local),'wb').write(b)
        records.append({'source_path':rel,'local_file':local,'url':f'https://github.com/ocsf/ocsf-schema/blob/{ref}/{rel}','sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b),'role':'schema_json'})
records.sort(key=lambda r:r['source_path'])
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'OCSF official schema JSON','ref':ref,'retrieved_at_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'source_urls':['https://github.com/ocsf/ocsf-schema','https://github.com/ocsf/ocsf-schema/tree/'+ref,url],'license':'Apache-2.0','scope':'schema JSON structure only; no prose/real logs/PII/detection routing/execution/graph wiring','files':records,'file_count':len(records)}
with open(os.path.join(out,'source-receipt.json'),'w',encoding='utf-8') as fp: json.dump(receipt,fp,ensure_ascii=False,indent=2,sort_keys=True); fp.write('\n')
print(f'downloaded OCSF schema tarball: ref={ref} json_files={len(records)} bytes={sum(r["bytes"] for r in records)} -> {out}')
PY
rm -rf "$TMP"
