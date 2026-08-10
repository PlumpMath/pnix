#!/usr/bin/env bash
# Library of Congress Linked Data selected vocabulary/root scheme snapshot.
# No individual authority dumps, person/name authority records, bibliographic records, prose notes, or graph wiring.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${LOC_LD_DEST:-$ROOT/ingest/metadata/loc-linked-data-vocab}"
rm -rf "$DEST"
mkdir -p "$DEST/raw"
python3 - "$DEST" <<'PY'
import datetime as dt, hashlib, json, pathlib, sys, urllib.request
out=pathlib.Path(sys.argv[1])
resources=[
    ('relators.rdf','https://id.loc.gov/vocabulary/relators.rdf','loc_vocabulary_relators_rdf'),
    ('identifiers.rdf','https://id.loc.gov/vocabulary/identifiers.rdf','loc_vocabulary_identifiers_rdf'),
    ('subjects.rdf','https://id.loc.gov/authorities/subjects.rdf','loc_authorities_subjects_scheme_root_rdf')
]
rows=[]
for name,url,role in resources:
    req=urllib.request.Request(url, headers={'User-Agent':'pnix-ingest-loc/0.1'})
    raw=urllib.request.urlopen(req, timeout=60).read()
    rel=pathlib.Path('raw')/name
    (out/rel).write_bytes(raw)
    rows.append({'source_path':str(rel),'relative_path':str(rel),'url':url,'sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw),'role':role})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'Library of Congress Linked Data vocabulary/root scheme RDF','retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'source_urls':['https://id.loc.gov/']+[u for _,u,_ in resources],'license':'LoC public domain / public authority data; selected scheme/vocabulary RDF only','scope':'selected official id.loc.gov vocabulary/root scheme RDF only; no individual authority record dumps, personal/name records, comment/definition prose, bibliographic records, linked payloads, or graph wiring','files':rows,'authority_record_payloads_downloaded':False,'personal_name_records_downloaded':False,'bibliographic_records_downloaded':False}
(out/'source-receipt.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True)+'\n', encoding='utf-8')
print(f'downloaded LoC linked-data vocab snapshot: files={len(rows)} -> {out}')
PY
