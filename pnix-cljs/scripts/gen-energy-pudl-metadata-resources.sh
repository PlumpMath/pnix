#!/usr/bin/env bash
# PUDL metadata raw snapshot -> pnix attrset source.
# 구조 identifier만 추출한다: file catalog, table/resource ids, field ids.
# prose descriptions, actual DB/parquet data rows, operational guidance는 저장하지 않는다.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${PUDL_SRC:-$ROOT/ingest/energy/pudl-metadata-resources}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/pudl-metadata-resources.generated.px}"
TABLE_LIMIT="${PUDL_TABLE_LIMIT:-600}"
FIELD_LIMIT="${PUDL_FIELD_LIMIT:-1200}"
if [[ ! -d "$SRC/raw/src/pudl/metadata" ]]; then
  echo "missing PUDL metadata snapshot: $SRC" >&2
  echo "run scripts/update-energy-pudl-metadata-resources.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$TABLE_LIMIT" "$FIELD_LIMIT" <<'PY'
import hashlib, json, pathlib, re, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2]); table_limit=int(sys.argv[3]); field_limit=int(sys.argv[4])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
raw=src/'raw'
pyfiles=sorted(raw.glob('src/pudl/metadata/**/*.py'))
common_non_table={
 'description','schema','fields','field_namespace','primary_key','foreign_key_rules','sources','etl_group','create_database_schema','encoder','type','constraints','unit','data_type','fuel_units','df','resource','resource_id','name','version','path','license','source','source_urls','working_partitions','partition','partitions','harvest','harvested',
 'additional_summary_text','additional_source_text','additional_details_text','additional_primary_key_text','usage_warnings','notes','contributors','keywords','license_url'
}
file_rows=[]; table_refs={}; field_refs={}
for p in pyfiles:
    rel=str(p.relative_to(raw))
    b=p.read_bytes(); text=b.decode('utf-8', errors='replace')
    role='resource_metadata' if '/resources/' in rel else 'core_metadata'
    file_rows.append({'relative_path':rel,'role':role,'module':rel[:-3].replace('/','.'),'size_bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()})
    if role=='resource_metadata':
        for m in re.finditer(r'(?m)^    ["\']([_a-z][a-z0-9_]{2,})["\']\s*:\s*\{', text):
            name=m.group(1)
            if name not in common_non_table and ('_' in name or name.startswith(('core','out','_core'))):
                table_refs.setdefault((name,rel), {'table':name,'source_file':rel})
        for block in re.finditer(r'["\']fields["\']\s*:\s*\[([^\]]*)\]', text, flags=re.S):
            for fm in re.finditer(r'["\']([a-zA-Z_][a-zA-Z0-9_]{1,})["\']', block.group(1)):
                field=fm.group(1)
                field_refs.setdefault((field,rel), {'field':field,'source_file':rel})
    if rel.endswith('fields.py'):
        for m in re.finditer(r'(?m)^    ["\']([a-zA-Z_][a-zA-Z0-9_]{1,})["\']\s*:\s*\{', text):
            field=m.group(1)
            if field not in common_non_table:
                field_refs.setdefault((field,rel), {'field':field,'source_file':rel})
tables=sorted(table_refs.values(), key=lambda r:(r['table'],r['source_file']))[:table_limit]
fields=sorted(field_refs.values(), key=lambda r:(r['field'],r['source_file']))[:field_limit]
obj={
 'schema':'energy.pudl_metadata_resources.v1',
 'source':{
   'name':'PUDL metadata resources',
   'license':'MIT software + CC-BY-4.0 data/docs; attribution required',
   'source_urls':['https://github.com/catalyst-cooperative/pudl','https://github.com/catalyst-cooperative/pudl/releases'],
   'receipt':receipt,
   'generator':'scripts/gen-energy-pudl-metadata-resources.sh',
   'scope':'official PUDL metadata resource file catalog and bounded identifier refs only; actual database/parquet rows and prose descriptions excluded'
 },
 'summary':{
   'ref':receipt.get('ref'),
   'file_count':len(file_rows),
   'core_file_count':sum(1 for r in file_rows if r['role']=='core_metadata'),
   'resource_file_count':sum(1 for r in file_rows if r['role']=='resource_metadata'),
   'resource_table_ref_count':len(tables),
   'resource_table_ref_total_found':len(table_refs),
   'field_ref_count':len(fields),
   'field_ref_total_found':len(field_refs),
   'table_limit':table_limit,
   'field_limit':field_limit,
   'actual_database_rows_ingested':False,
   'parquet_or_sqlite_payloads_ingested':False,
   'prose_descriptions_ingested':False,
   'operational_dispatch_guidance_ingested':False,
   'grid_security_sensitive_payload_ingested':False,
   'forecast_or_trading_advice_ingested':False,
   'mirror_graph_wiring':False,
 },
 'files':file_rows,
 'resource_table_refs':tables,
 'field_refs':fields,
}
def pnix(v, indent=0):
    sp='  '*indent
    if v is None: return 'null'
    if v is True: return 'true'
    if v is False: return 'false'
    if isinstance(v,(int,float)): return json.dumps(v)
    if isinstance(v,str): return json.dumps(v, ensure_ascii=False)
    if isinstance(v,list):
        if not v: return '[ ]'
        return '[\n' + ''.join(sp+'  '+pnix(x, indent+1)+'\n' for x in v) + sp + ']'
    if isinstance(v,dict):
        if not v: return '{ }'
        return '{\n' + '\n'.join(sp+'  '+json.dumps(str(k), ensure_ascii=False)+' = '+pnix(v[k], indent+1)+';' for k in sorted(v)) + '\n' + sp + '}'
    return json.dumps(str(v), ensure_ascii=False)
content='# stdlib/lib/corpus/pudl-metadata-resources.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-energy-pudl-metadata-resources.sh && scripts/gen-energy-pudl-metadata-resources.sh\n'
content+='# 범위: PUDL metadata resource file catalog + table/field identifier refs only. DB/parquet/prose/dispatch/security/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(content, encoding='utf-8')
print(f'generated {out}: files={len(file_rows)} tables={len(tables)}/{len(table_refs)} fields={len(fields)}/{len(field_refs)} bytes={len(content.encode())}')
PY
