#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/bio/iof-ontology-catalog"
mkdir -p "$OUT/raw"
UA="pnix-ingest/1.0 (IOF ontology structural catalog only; no graph wiring)"
REPO_API="${IOF_REPO_API:-https://api.github.com/repos/iofoundry/ontology}"
curl -fsSL --retry 2 --max-time 30 -A "$UA" "$REPO_API" -o "$OUT/repo.json"
BRANCH="${IOF_REF:-$(python3 - "$OUT/repo.json" <<'PY'
import json,sys
print(json.load(open(sys.argv[1])).get('default_branch','master'))
PY
)}"
curl -fsSL --retry 2 --max-time 60 -A "$UA" "https://api.github.com/repos/iofoundry/ontology/git/trees/$BRANCH?recursive=1" -o "$OUT/tree.json"
curl -fsSL --retry 2 --max-time 30 -A "$UA" "https://raw.githubusercontent.com/iofoundry/ontology/$BRANCH/LICENSE" -o "$OUT/LICENSE"
python3 - "$OUT" "$BRANCH" "${IOF_RDF_FILE_LIMIT:-28}" <<'PY'
import json,pathlib,subprocess,sys,hashlib,datetime
out=pathlib.Path(sys.argv[1]); branch=sys.argv[2]; limit=int(sys.argv[3])
tree=json.load(open(out/'tree.json')).get('tree',[])
def include(path,size):
    low=path.lower()
    if not low.endswith('.rdf'): return False
    if any(x in low for x in ['/cache/','cache/','/test/','/testing/','/examples/','/addenda/examples/','migration/']): return False
    if path.startswith('cache/'): return False
    return (size or 0) <= int(__import__('os').environ.get('IOF_RDF_MAX_BYTES','450000'))
files=[x for x in tree if x.get('type')=='blob' and include(x.get('path',''),x.get('size',0))]
files=files[:limit]
for x in files:
    p=x['path']; dest=out/'raw'/p.replace('/','__')
    url=f'https://raw.githubusercontent.com/iofoundry/ontology/{branch}/{p}'
    subprocess.run(['curl','-fsSL','--retry','2','--max-time','60','-A','pnix-ingest/1.0',url,'-o',str(dest)],check=True)
manifest_files=[]
for p in sorted(out.rglob('*')):
    if p.is_file() and p.name!='source-manifest.json':
        manifest_files.append({'path':str(p.relative_to(out)),'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
manifest={'schema':'pnix.source_manifest.v1','source_id':'iof-ontology-catalog','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'ref':branch,'downloaded_rdf_files':[x['path'] for x in files],'files':manifest_files,'policy':'IOF repository/tree and RDF structural token catalog only; RDF bodies, labels/comments/definitions, examples/tests/cache ontologies and graph wiring excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'ref':branch,'rdf_files':len(files),'out':str(out)},ensure_ascii=False))
PY
