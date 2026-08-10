#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DST="$ROOT/ingest/license/scancode-licensedb"
REF="${SCANCODE_LICENSEDB_REF:-main}"
mkdir -p "$DST"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/tree.json" "https://api.github.com/repos/aboutcode-org/scancode-licensedb/git/trees/$REF?recursive=1"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/apache-2.0.LICENSE" "https://raw.githubusercontent.com/aboutcode-org/scancode-licensedb/$REF/apache-2.0.LICENSE"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/cc-by-4.0.LICENSE" "https://raw.githubusercontent.com/aboutcode-org/scancode-licensedb/$REF/cc-by-4.0.LICENSE"
curl -L --fail --retry 3 --connect-timeout 20 -o "$DST/index.json" "https://raw.githubusercontent.com/aboutcode-org/scancode-licensedb/$REF/docs/index.json"
python3 - "$DST" "$REF" <<'PY'
import json, pathlib, sys, hashlib, datetime
root=pathlib.Path(sys.argv[1]); ref=sys.argv[2]
data=json.loads((root/'index.json').read_text())
files=[]
for name in ['tree.json','apache-2.0.LICENSE','cc-by-4.0.LICENSE','index.json']:
    p=root/name; b=p.read_bytes(); files.append({'path':name,'sha256':hashlib.sha256(b).hexdigest(),'bytes':len(b)})
(root/'source-manifest.json').write_text(json.dumps({'schema':'pnix.ingest_source_manifest.v1','source_id':'scancode-licensedb','source_name':'ScanCode LicenseDB index','license_id':'CC-BY-4.0 metadata / Apache-2.0 tooling','ref':ref,'retrieved_at':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':['https://github.com/aboutcode-org/scancode-licensedb','https://raw.githubusercontent.com/aboutcode-org/scancode-licensedb/'+ref+'/docs/index.json'],'records':len(data),'files':files,'policy':'Index metadata only. Exclude license text bodies, HTML/YAML/full JSON bodies, legal interpretation, compatibility/compliance advice, package-specific decisions, graph wiring.'},indent=2,ensure_ascii=False),encoding='utf-8')
print(f'updated {root}/source-manifest.json records={len(data)}')
PY
