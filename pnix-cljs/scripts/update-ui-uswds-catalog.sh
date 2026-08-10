#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DST="$ROOT/ingest/ui/uswds-catalog"
REF="${USWDS_REF:-}"
if [ -z "$REF" ]; then
  REF="$(curl -L --fail --silent https://api.github.com/repos/uswds/uswds/releases/latest | python3 -c 'import json,sys; print(json.load(sys.stdin)["tag_name"])')"
fi
mkdir -p "$DST"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/tree.json" "https://api.github.com/repos/uswds/uswds/git/trees/$REF?recursive=1"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/release.json" "https://api.github.com/repos/uswds/uswds/releases/tags/$REF"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/LICENSE.md" "https://raw.githubusercontent.com/uswds/uswds/$REF/LICENSE.md"
python3 - "$DST" "$REF" <<'PY'
import json, pathlib, sys, hashlib, datetime
root=pathlib.Path(sys.argv[1]); ref=sys.argv[2]
files=[]
for name in ['tree.json','release.json','LICENSE.md']:
    p=root/name; b=p.read_bytes(); files.append({'path':name,'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b)})
(root/'source-manifest.json').write_text(json.dumps({'schema':'pnix.ingest_source_manifest.v1','source_id':'uswds-catalog','source_name':'USWDS component/token catalog','license_id':'US-PD/CC0-1.0 with third-party asset exclusions','ref':ref,'retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':['https://github.com/uswds/uswds','https://api.github.com/repos/uswds/uswds/git/trees/'+ref+'?recursive=1'],'files':files,'policy':'Tree/path metadata only. Exclude fonts/icons/images/normalize/source bodies/docs prose/examples/runtime JS/graph wiring.'},indent=2),encoding='utf-8')
print(f'updated {root}/source-manifest.json ref={ref} files={len(files)}')
PY
