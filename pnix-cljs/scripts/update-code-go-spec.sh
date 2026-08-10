#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DST="$ROOT/ingest/code/go-spec"
mkdir -p "$DST"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/spec.html" "https://go.dev/ref/spec"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/LICENSE" "https://raw.githubusercontent.com/golang/go/master/LICENSE"
python3 - "$DST" <<'PY'
import json, pathlib, hashlib, datetime
root=pathlib.Path(__import__('sys').argv[1])
files=[]
for name in ['spec.html','LICENSE']:
    p=root/name; b=p.read_bytes(); files.append({'path':name,'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b)})
(root/'source-manifest.json').write_text(json.dumps({
  'schema':'pnix.ingest_source_manifest.v1',
  'source_id':'go-spec',
  'source_name':'Go Language Specification',
  'license_id':'BSD-3-Clause-style Go license',
  'retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),
  'source_urls':['https://go.dev/ref/spec','https://raw.githubusercontent.com/golang/go/master/LICENSE'],
  'files':files,
  'policy':'Official spec structural headings and EBNF productions only. Exclude prose, examples, source corpora, execution, graph wiring.'
},indent=2,ensure_ascii=False),encoding='utf-8')
print(f'updated {root}/source-manifest.json files={len(files)}')
PY
