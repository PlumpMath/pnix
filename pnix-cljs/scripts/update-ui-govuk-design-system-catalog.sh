#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DST="$ROOT/ingest/ui/govuk-design-system-catalog"
REF="${GOVUK_DESIGN_SYSTEM_REF:-main}"
mkdir -p "$DST"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/tree.json" "https://api.github.com/repos/alphagov/govuk-design-system/git/trees/$REF?recursive=1"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/LICENSE" "https://raw.githubusercontent.com/alphagov/govuk-design-system/$REF/LICENSE"
python3 - "$DST" "$REF" <<'PY'
import json, pathlib, sys, hashlib, datetime
root=pathlib.Path(sys.argv[1]); ref=sys.argv[2]
files=[]
for name in ['tree.json','LICENSE']:
    p=root/name; b=p.read_bytes(); files.append({'path':name,'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b)})
(root/'source-manifest.json').write_text(json.dumps({'schema':'pnix.ingest_source_manifest.v1','source_id':'govuk-design-system-catalog','source_name':'GOV.UK Design System catalog','license_id':'MIT','ref':ref,'retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':['https://github.com/alphagov/govuk-design-system','https://api.github.com/repos/alphagov/govuk-design-system/git/trees/'+ref+'?recursive=1'],'files':files,'policy':'Tree/path metadata only. Exclude docs prose, template/style/script bodies, assets, examples, runtime rendering, graph wiring.'},indent=2),encoding='utf-8')
print(f'updated {root}/source-manifest.json ref={ref} files={len(files)}')
PY
