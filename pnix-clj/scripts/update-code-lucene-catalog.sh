#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DST="$ROOT/ingest/code/lucene-catalog"
REF="${LUCENE_REF:-}"
if [ -z "$REF" ]; then
  REF="$(curl -L --fail --silent https://api.github.com/repos/apache/lucene/releases/latest | python3 -c 'import json,sys; print(json.load(sys.stdin)["tag_name"])')"
fi
mkdir -p "$DST"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/tree.json" "https://api.github.com/repos/apache/lucene/git/trees/$REF?recursive=1"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/release.json" "https://api.github.com/repos/apache/lucene/releases/tags/$REF"
python3 - "$DST" "$REF" <<'PY'
import json, pathlib, sys, hashlib, datetime
root=pathlib.Path(sys.argv[1]); ref=sys.argv[2]
files=[]
for name in ['tree.json','release.json']:
    p=root/name
    b=p.read_bytes()
    files.append({'path':name,'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b)})
(root/'source-manifest.json').write_text(json.dumps({
  'schema':'pnix.ingest_source_manifest.v1',
  'source_id':'lucene-catalog',
  'source_name':'Apache Lucene release source catalog',
  'license_id':'Apache-2.0',
  'ref':ref,
  'retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),
  'source_urls':['https://github.com/apache/lucene','https://api.github.com/repos/apache/lucene/git/trees/'+ref+'?recursive=1'],
  'files':files,
  'policy':'Tree/path metadata only. Source bodies, docs prose, tests/testdata, benchmark/ranking values, execution, graph wiring excluded.'
},indent=2),encoding='utf-8')
print(f'updated {root}/source-manifest.json ref={ref} files={len(files)}')
PY
