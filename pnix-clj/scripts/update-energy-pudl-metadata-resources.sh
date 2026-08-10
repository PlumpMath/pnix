#!/usr/bin/env bash
# PUDL official release metadata files -> local raw snapshot.
# 범위: src/pudl/metadata/*.py 일부 + src/pudl/metadata/resources/*.py 파일만 저장.
# 실제 PUDL DB/Parquet/SQLite/data release payload는 다운로드하지 않는다.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REF="${PUDL_REF:-v2026.6.0}"
DEST="${PUDL_DEST:-$ROOT/ingest/energy/pudl-metadata-resources}"
RAW="$DEST/raw"
mkdir -p "$RAW/src/pudl/metadata/resources"
python3 - "$REF" "$DEST" <<'PY'
import datetime as dt, hashlib, json, pathlib, sys, urllib.request
ref=sys.argv[1]; dest=pathlib.Path(sys.argv[2]); raw=dest/'raw'
api='https://api.github.com/repos/catalyst-cooperative/pudl/contents'
core_keep={'__init__.py','codes.py','constants.py','descriptions.py','dfs.py','enums.py','fields.py','labels.py','sources.py'}
def get_json(url):
    req=urllib.request.Request(url, headers={'Accept':'application/vnd.github+json','User-Agent':'pnix-pudl-metadata-ingest'})
    with urllib.request.urlopen(req, timeout=60) as r: return json.load(r)
def get_bytes(url):
    req=urllib.request.Request(url, headers={'User-Agent':'pnix-pudl-metadata-ingest'})
    with urllib.request.urlopen(req, timeout=60) as r: return r.read()
core_index=get_json(f'{api}/src/pudl/metadata?ref={ref}')
res_index=get_json(f'{api}/src/pudl/metadata/resources?ref={ref}')
(dest/'metadata-index.json').write_text(json.dumps(core_index, ensure_ascii=False, indent=2, sort_keys=True)+'\n', encoding='utf-8')
(dest/'resources-index.json').write_text(json.dumps(res_index, ensure_ascii=False, indent=2, sort_keys=True)+'\n', encoding='utf-8')
files=[]
for item in core_index:
    if item.get('type')=='file' and item.get('name') in core_keep:
        files.append(('core_metadata', item, pathlib.Path('src/pudl/metadata')/item['name']))
for item in res_index:
    if item.get('type')=='file' and item.get('name','').endswith('.py'):
        files.append(('resource_metadata', item, pathlib.Path('src/pudl/metadata/resources')/item['name']))
records=[]
for role,item,rel in files:
    b=get_bytes(item['download_url'])
    p=raw/rel
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_bytes(b)
    records.append({
        'role': role,
        'name': item['name'],
        'relative_path': str(rel),
        'download_url': item['download_url'],
        'html_url': item.get('html_url'),
        'github_sha': item.get('sha'),
        'size_bytes': len(b),
        'sha256': hashlib.sha256(b).hexdigest(),
    })
receipt={
    'schema':'pnix.ingest.source_receipt.v1',
    'source':'PUDL metadata resources',
    'ref':ref,
    'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),
    'source_urls':['https://github.com/catalyst-cooperative/pudl','https://github.com/catalyst-cooperative/pudl/releases'],
    'api_urls':[f'{api}/src/pudl/metadata?ref={ref}', f'{api}/src/pudl/metadata/resources?ref={ref}'],
    'license':'MIT software + CC-BY-4.0 data/docs; attribution required; no share-alike',
    'scope':'official PUDL metadata Python modules only; no DB/parquet payload rows, documentation prose, dispatch/control guidance, or graph/mirror wiring',
    'core_files_downloaded':sum(1 for r in records if r['role']=='core_metadata'),
    'resource_files_downloaded':sum(1 for r in records if r['role']=='resource_metadata'),
    'files':records,
}
(dest/'source-receipt.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True)+'\n', encoding='utf-8')
print(f'downloaded PUDL metadata {ref}: files={len(records)} core={receipt["core_files_downloaded"]} resources={receipt["resource_files_downloaded"]} -> {dest}')
PY
