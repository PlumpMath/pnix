#!/usr/bin/env bash
# CPC official scheme bulk snapshot.
# Downloads small official scheme/title/symbol artifacts only. No CPC definitions prose/PDF, patent docs, MCF assignment payloads, or graph wiring.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${CPC_SCHEME_DEST:-$ROOT/ingest/patent/uspto-cpc-scheme}"
VERSION="${CPC_SCHEME_VERSION:-202605}"
BASE="${CPC_SCHEME_BASE:-https://www.cooperativepatentclassification.org}"
rm -rf "$DEST"
mkdir -p "$DEST/raw"
python3 - "$DEST" "$BASE" "$VERSION" <<'PY'
import datetime as dt, hashlib, json, pathlib, sys, urllib.request
out=pathlib.Path(sys.argv[1]); base=sys.argv[2].rstrip('/'); version=sys.argv[3]
files=[
    ('CPCSchemeSchema17.xsd','/sites/default/files/attachments/b0cebc06-677b-4243-9c9a-adf7300b979f/CPCSchemeSchema17.xsd','cpc_scheme_xsd'),
    (f'CPCSymbolList{version}.zip',f'/sites/default/files/cpc/bulk/CPCSymbolList{version}.zip','cpc_symbol_list_zip'),
    (f'CPCValidityFile{version}.zip',f'/sites/default/files/cpc/bulk/CPCValidityFile{version}.zip','cpc_validity_file_zip'),
    (f'CPCTitleList{version}.zip',f'/sites/default/files/cpc/bulk/CPCTitleList{version}.zip','cpc_title_list_zip')
]
rows=[]
headers={'User-Agent':'Mozilla/5.0 pnix-ingest-cpc/0.1'}
for name,path,role in files:
    url=base+path
    req=urllib.request.Request(url, headers=headers)
    raw=urllib.request.urlopen(req, timeout=120).read()
    rel=pathlib.Path('raw')/name
    (out/rel).write_bytes(raw)
    rows.append({'source_path':str(rel),'relative_path':str(rel),'url':url,'sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw),'role':role})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'Cooperative Patent Classification official scheme bulk data','version':version,'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'source_urls':['https://www.cooperativepatentclassification.org/cpcSchemeAndDefinitions/bulk','https://www.cooperativepatentclassification.org/cpcSchemeAndDefinitions/CPCopenLinkedData','https://www.uspto.gov/web/offices/pac/mpep/s905.html'],'license':'CPC official open data / public classification scheme; scheme/title/symbol metadata only','scope':'official CPC scheme schema, symbol list, validity file, and title list only; no patent documents, CPC definitions prose/PDF, MCF patent-document assignment payloads, linked payloads, or graph wiring','files':rows,'cpc_definitions_downloaded':False,'patent_documents_downloaded':False,'mcf_assignment_payloads_downloaded':False}
(out/'source-receipt.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True)+'\n', encoding='utf-8')
print(f'downloaded CPC scheme snapshot: version={version} files={len(rows)} -> {out}')
PY
