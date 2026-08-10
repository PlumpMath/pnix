#!/usr/bin/env bash
# QIF qif-community XML schema snapshot.
# Uses official qif-community repo. Ingests only Boost-licensed CPP-Kramer XSD schema files.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${QIF_DEST:-$ROOT/ingest/quality/qif}"
REF="${QIF_REF:-master}"
TMP="${TMPDIR:-/tmp}/pnix-qif-community-$$"
rm -rf "$TMP"
git -c advice.detachedHead=false clone --depth 1 --branch "$REF" https://github.com/QualityInformationFramework/qif-community.git "$TMP" >/dev/null 2>&1
python3 - "$TMP" "$DEST" "$REF" <<'PY'
import datetime as dt, hashlib, json, pathlib, shutil, sys
src=pathlib.Path(sys.argv[1]); dest=pathlib.Path(sys.argv[2]); ref=sys.argv[3]
schema_dir=src/'bindings/CPP-Kramer/schema'
paths=[p.relative_to(src).as_posix() for p in sorted(schema_dir.glob('*.xsd')) if p.name.endswith('.xsd') and not p.name.endswith('.xsdOrig')]
records=[]
for path in ['LICENSE.md']+paths:
    raw=(src/path).read_bytes()
    rel=pathlib.Path('raw')/path
    out=dest/rel; out.parent.mkdir(parents=True,exist_ok=True); out.write_bytes(raw)
    role='license' if path=='LICENSE.md' else 'xsd_schema'
    records.append({'source_path':path,'relative_path':str(rel),'url':f'https://github.com/QualityInformationFramework/qif-community/blob/{ref}/{path}','sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw),'role':role})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'Quality Information Framework qif-community XML schema metadata','ref':ref,'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'source_urls':['https://github.com/QualityInformationFramework/qif-community','https://qifstandards.org/about-qif/'],'license':'Boost Software License 1.0 for qif-community; CodeSynthesis binding paths excluded','scope':'Boost-licensed CPP-Kramer XSD schema structure only; no CodeSynthesis bindings/generated source/sample instances/measurement values/process guidance/execution/graph wiring','files':records,'xsd_file_count':len(paths)}
(dest/'source-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2,sort_keys=True)+'\n',encoding='utf-8')
print(f'downloaded QIF schema snapshot: ref={ref} xsd_files={len(paths)} -> {dest}')
PY
rm -rf "$TMP"
