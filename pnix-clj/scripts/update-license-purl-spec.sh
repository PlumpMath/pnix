#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DST="$ROOT/ingest/license/purl-spec"
REF="${PURL_SPEC_REF:-main}"
mkdir -p "$DST/types" "$DST/schemas"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/tree.json" "https://api.github.com/repos/package-url/purl-spec/git/trees/$REF?recursive=1"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/LICENSE" "https://raw.githubusercontent.com/package-url/purl-spec/$REF/LICENSE"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/purl-types-index.json" "https://raw.githubusercontent.com/package-url/purl-spec/$REF/purl-types-index.json"
python3 - "$DST" "$REF" <<'PY'
import json, pathlib, sys, hashlib, datetime, urllib.request
root=pathlib.Path(sys.argv[1]); ref=sys.argv[2]
types=json.loads((root/'purl-types-index.json').read_text())
for t in types:
    url=f'https://raw.githubusercontent.com/package-url/purl-spec/{ref}/types/{t}-definition.json'
    with urllib.request.urlopen(urllib.request.Request(url,headers={'User-Agent':'pnix-ingest/1.0'}),timeout=30) as r:
        (root/'types'/f'{t}-definition.json').write_bytes(r.read())
for name in ['purl-type-definition.schema-1.0.json','purl-type-definition.schema-1.1.json','purl-types-index.schema-1.0.json','purl-types-index.schema-1.1.json']:
    url=f'https://raw.githubusercontent.com/package-url/purl-spec/{ref}/schemas/{name}'
    with urllib.request.urlopen(urllib.request.Request(url,headers={'User-Agent':'pnix-ingest/1.0'}),timeout=30) as r:
        (root/'schemas'/name).write_bytes(r.read())
files=[]
for p in [root/'tree.json',root/'LICENSE',root/'purl-types-index.json']+sorted((root/'types').glob('*.json'))+sorted((root/'schemas').glob('*.json')):
    b=p.read_bytes(); files.append({'path':str(p.relative_to(root)),'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b)})
(root/'source-manifest.json').write_text(json.dumps({'schema':'pnix.ingest_source_manifest.v1','source_id':'purl-spec','source_name':'package-url specification','license_id':'MIT','ref':ref,'retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':['https://github.com/package-url/purl-spec'],'type_count':len(types),'files':files,'policy':'Machine-readable structural metadata only. Exclude docs prose, examples, real package registry data, dependency/license package data, tests, graph wiring.'},indent=2,ensure_ascii=False),encoding='utf-8')
print(f'updated {root}/source-manifest.json ref={ref} types={len(types)}')
PY
