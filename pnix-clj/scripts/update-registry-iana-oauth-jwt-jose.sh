#!/usr/bin/env bash
# IANA OAuth/JWT/JOSE registries snapshot updater.
set -euo pipefail
ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
OUTDIR="$ROOT/ingest/registry/iana-oauth-jwt-jose"
mkdir -p "$OUTDIR"
python3 - "$OUTDIR" <<'PY'
import datetime, hashlib, json, sys, urllib.request
outdir=sys.argv[1]
sources=[
  ('oauth-parameters','https://www.iana.org/assignments/oauth-parameters/oauth-parameters.xml'),
  ('jwt','https://www.iana.org/assignments/jwt/jwt.xml'),
  ('jose','https://www.iana.org/assignments/jose/jose.xml'),
]
items=[]
for sid,url in sources:
    path=f'{outdir}/{sid}.xml'
    with urllib.request.urlopen(url, timeout=30) as r:
        raw=r.read()
    open(path,'wb').write(raw)
    items.append({'source_id':sid,'source_url':url,'source_path':f'{sid}.xml','source_sha256':hashlib.sha256(raw).hexdigest()})
obj={
  'schema':'pnix.ingest.source_manifest.v1',
  'source_id':'iana-oauth-jwt-jose',
  'project':'IANA OAuth/JWT/JOSE Registries',
  'snapshot_kind':'latest-xml-snapshot-bundle',
  'retrieved_at_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),
  'license':'IANA any-purpose registry terms',
  'version_policy':'No release tags are exposed. Each fetched XML bundle is content-addressed and append-only in redb; rerun this script to capture newer registry snapshots.',
  'sources':items,
}
open(f'{outdir}/manifest.json','w',encoding='utf-8').write(json.dumps(obj,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(obj,ensure_ascii=False))
PY
