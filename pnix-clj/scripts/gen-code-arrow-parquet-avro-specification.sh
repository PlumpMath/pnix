#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/ingest/code/arrow-parquet-avro-specification"
OUT="$ROOT/stdlib/lib/corpus/arrow-parquet-avro-specification.generated.px"
python3 - "$SRC" "$OUT" <<'PY'
import pathlib, sys, re, json, hashlib
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
def esc(s): return '"'+str(s).replace('$','DOLLAR_SIGN').replace('\\','\\\\').replace('"','\\"').replace('\n',' ').replace('\r','')+'"'
def lit(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,list): return '[ ' + ' '.join(lit(x) for x in v) + ' ]'
    if isinstance(v,dict): return '{ ' + ' '.join(f'{k} = {lit(val)};' for k,val in v.items()) + ' }'
    return esc('' if v is None else v)
manifest={}
if (src/'source-manifest.json').exists(): manifest=json.loads((src/'source-manifest.json').read_text())
files=[]
for p in sorted(src.rglob('*')):
    if p.is_file() and p.name!='source-manifest.json': files.append({'path':str(p.relative_to(src)),'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
arrow_types=[]; arrow_fields=[]
for p in sorted((src/'arrow/format').glob('*.fbs')):
    rel=str(p.relative_to(src)); current=None
    for line in p.read_text(encoding='utf-8',errors='replace').splitlines():
        line=line.split('//',1)[0].strip()
        m=re.match(r'^(table|struct|enum|union)\s+([A-Za-z_][A-Za-z0-9_]*)', line)
        if m:
            current=m.group(2); arrow_types.append({'file':rel,'kind':m.group(1),'name':current}); continue
        fm=re.match(r'^([A-Za-z_][A-Za-z0-9_]*)\s*:\s*([^=;]+)', line)
        if current and fm: arrow_fields.append({'file':rel,'owner':current,'field':fm.group(1),'type':fm.group(2).strip()[:120]})
parquet_types=[]; parquet_fields=[]
p=src/'parquet/src/main/thrift/parquet.thrift'
if p.exists():
    current=None
    for line in p.read_text(encoding='utf-8',errors='replace').splitlines():
        line=line.split('//',1)[0].strip()
        m=re.match(r'^(struct|enum)\s+([A-Za-z_][A-Za-z0-9_]*)', line)
        if m: current=m.group(2); parquet_types.append({'kind':m.group(1),'name':current}); continue
        fm=re.match(r'^(\d+)\s*:\s*(optional|required)?\s*([^\s]+)\s+([A-Za-z_][A-Za-z0-9_]*)', line)
        if current and fm: parquet_fields.append({'owner':current,'id':fm.group(1),'req':fm.group(2) or 'default','type':fm.group(3),'field':fm.group(4)})
avro_grammar=[]
g=src/'avro/share/idl_grammar/org/apache/avro/idl/Idl.g4'
if g.exists():
    for line in g.read_text(encoding='utf-8',errors='replace').splitlines():
        line=line.strip()
        m=re.match(r'^([A-Za-z_][A-Za-z0-9_]*)\s*:', line)
        if m: avro_grammar.append({'rule':m.group(1)})
avro_records=[]; avro_fields=[]; avro_protocols=[]
def walk_schema(obj, rel, owner=''):
    if isinstance(obj,dict):
        typ=obj.get('type')
        name=obj.get('name')
        if typ in ('record','enum','fixed') or ('fields' in obj and name):
            avro_records.append({'file':rel,'kind':str(typ),'name':str(name or owner),'namespace':str(obj.get('namespace',''))})
            owner=str(name or owner)
        if isinstance(obj.get('fields'),list):
            for f in obj['fields']:
                if isinstance(f,dict): avro_fields.append({'file':rel,'owner':owner,'field':str(f.get('name','')),'type':json.dumps(f.get('type',''),sort_keys=True)[:160]})
        for v in obj.values(): walk_schema(v, rel, owner)
    elif isinstance(obj,list):
        for v in obj: walk_schema(v, rel, owner)
for p in sorted((src/'avro').rglob('*.avsc'))+sorted((src/'avro').rglob('*.avpr')):
    rel=str(p.relative_to(src))
    try:
        data=json.loads(p.read_text(encoding='utf-8'))
        if p.suffix=='.avpr': avro_protocols.append({'file':rel,'protocol':str(data.get('protocol','')),'namespace':str(data.get('namespace',''))})
        walk_schema(data, rel)
    except Exception as e:
        avro_records.append({'file':rel,'kind':'parse_error','name':str(e)[:120],'namespace':''})
obj={'schema':'db.arrow_parquet_avro.specification.v1','source':'Apache Arrow / Parquet / Avro structural schemas','license':'Apache-2.0','summary':{'files':len(files),'arrow_types':len(arrow_types),'arrow_fields':len(arrow_fields),'parquet_types':len(parquet_types),'parquet_fields':len(parquet_fields),'avro_grammar_rules':len(avro_grammar),'avro_records':len(avro_records),'avro_fields':len(avro_fields),'avro_protocols':len(avro_protocols)},'policy':'official schema/IDL structure only; real data files, examples/tests, customer schemas, schema registry subjects, docs prose, execution, graph wiring excluded','manifest':manifest,'files':files,'arrow_types':arrow_types,'arrow_fields':arrow_fields[:1200],'parquet_types':parquet_types,'parquet_fields':parquet_fields[:1200],'avro_grammar_rules':avro_grammar,'avro_records':avro_records,'avro_fields':avro_fields,'avro_protocols':avro_protocols}
out.write_text('# GENERATED by scripts/gen-code-arrow-parquet-avro-specification.sh. Do not edit. Gitignored.\n# Source: Apache Arrow FlatBuffers, Parquet Thrift, Avro grammar/core schemas.\n# Policy: structural schema metadata only; data/examples/customer schemas/execution excluded.\n'+lit(obj)+'\n',encoding='utf-8')
print(f'generated {out}: arrow_types={len(arrow_types)} parquet_types={len(parquet_types)} avro_records={len(avro_records)} bytes={out.stat().st_size}')
PY
