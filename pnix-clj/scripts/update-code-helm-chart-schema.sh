#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEST="${HELM_CHART_SCHEMA_DEST:-$ROOT/ingest/code/helm-chart-schema}"
REF="${HELM_CHART_SCHEMA_REF:-v4.2.2}"
TMP="${TMPDIR:-/tmp}/pnix-helm-chart-schema-$$"
rm -rf "$TMP"; mkdir -p "$TMP" "$DEST/raw"
URL="https://github.com/helm/helm/archive/refs/tags/$REF.tar.gz"
curl -fsSL "$URL" -o "$TMP/helm.tar.gz"
tar -xzf "$TMP/helm.tar.gz" -C "$TMP"
SRC="$TMP/helm-${REF#v}"
python3 - "$DEST" "$REF" "$URL" "$SRC" <<'PY'
import hashlib,json,os,sys,time
out,ref,url,src=sys.argv[1:]
selected=[]
for rel_dir in ['pkg/chart/v2','pkg/chart']:
    d=os.path.join(src,rel_dir)
    if not os.path.isdir(d): continue
    for fn in sorted(os.listdir(d)):
        if fn.endswith('.go') and not fn.endswith('_test.go'):
            selected.append(f'{rel_dir}/{fn}')
rawdir=os.path.join(out,'raw'); os.makedirs(rawdir,exist_ok=True)
records=[]
for rel in selected:
    p=os.path.join(src,rel); b=open(p,'rb').read(); local=rel.replace('/','__')
    open(os.path.join(rawdir,local),'wb').write(b)
    records.append({'source_path':rel,'local_file':local,'url':f'https://github.com/helm/helm/blob/{ref}/{rel}','sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b),'role':'go_model_source'})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'Helm chart model structural metadata','ref':ref,'retrieved_at_utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'source_urls':['https://github.com/helm/helm','https://github.com/helm/helm/tree/'+ref+'/pkg/chart/v2',url],'license':'Apache-2.0','scope':'Go model struct/tag identifiers only; no source bodies/tests/real charts/templates/values/manifests/cluster secrets/execution/graph wiring','files':records}
with open(os.path.join(out,'source-receipt.json'),'w',encoding='utf-8') as fp: json.dump(receipt,fp,ensure_ascii=False,indent=2,sort_keys=True); fp.write('\n')
print(f'downloaded Helm chart model: ref={ref} files={len(records)} bytes={sum(r["bytes"] for r in records)} -> {out}')
PY
rm -rf "$TMP"
