#!/usr/bin/env bash
# CloudEvents spec snapshot -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${CLOUDEVENTS_SPEC_SRC:-$ROOT/ingest/code/cloudevents-specification}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/cloudevents-specification.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing CloudEvents snapshot: $SRC" >&2
  echo "run scripts/update-code-cloudevents-specification.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import json, pathlib, re, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
source_files=[]; table_rows=[]; tokens=[]; json_props=[]; avro_fields=[]; proto_fields=[]
seen=set(); TOKEN_RE=re.compile(r'`([^`\n]{1,96})`')
for f in receipt.get('files',[]):
    if f.get('role') not in ('spec_doc','format_schema'): continue
    path=f['source_path']; rel=f['relative_path']; p=src/rel
    source_files.append({'source_path':path,'role':f['role'],'sha256':f['sha256'],'size_bytes':f['size_bytes']})
    if path.endswith('.md'):
        text=p.read_text(encoding='utf-8',errors='replace'); section=''
        for line_no,line in enumerate(text.splitlines(),1):
            s=line.strip()
            if s.startswith('#'):
                title=s.lstrip('#').strip()
                if len(title)<=80: section=title
                continue
            if s.startswith('|') and s.endswith('|'):
                cells=[c.strip() for c in s.strip('|').split('|')]
                if not cells or all(set(c.replace(':','').replace('-','').strip()) <= {'-'} for c in cells): continue
                joined=' '.join(cells)
                if len(joined)<=360 and all(len(c)<=140 for c in cells):
                    table_rows.append({'source_path':path,'line':line_no,'section':section,'cells':cells})
            for m in TOKEN_RE.finditer(line):
                tok=m.group(1).strip()
                if not tok or len(tok)>96 or '://' in tok or tok.count(' ')>2: continue
                key=(path,tok)
                if key not in seen:
                    seen.add(key); tokens.append({'source_path':path,'section':section,'token':tok})
    elif path.endswith('.json'):
        data=json.loads(p.read_text(encoding='utf-8'))
        req=set(data.get('required') or [])
        for name,prop in (data.get('properties') or {}).items():
            json_props.append({'source_path':path,'property':name,'type':prop.get('type'), 'format':prop.get('format'), 'required':name in req})
    elif path.endswith('.avsc'):
        data=json.loads(p.read_text(encoding='utf-8'))
        for field in data.get('fields') or []:
            typ=field.get('type')
            avro_fields.append({'source_path':path,'field':field.get('name'),'type':json.dumps(typ,ensure_ascii=False,sort_keys=True) if not isinstance(typ,str) else typ})
    elif path.endswith('.proto'):
        text=p.read_text(encoding='utf-8',errors='replace')
        pkg=(re.search(r'\bpackage\s+([A-Za-z0-9_.]+)\s*;',text) or [None,None])[1]
        for m in re.finditer(r'(?m)^\s*(optional|required|repeated)?\s*([A-Za-z_][A-Za-z0-9_.<>]*)\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*([0-9]+)',text):
            proto_fields.append({'source_path':path,'package':pkg,'label':m.group(1) or 'singular','type':m.group(2),'name':m.group(3),'number':int(m.group(4))})
obj={'schema':'api.cloudevents.specification.v1','source':{'name':'CloudEvents official specification documents and format schemas','license':'Apache-2.0','source_urls':['https://github.com/cloudevents/spec','https://github.com/cloudevents/spec/releases'],'receipt':receipt,'generator':'scripts/gen-code-cloudevents-specification.sh','scope':'official spec/binding/format structural metadata only; prose bodies/SDK/primer/webhook/subscriptions/payloads/endpoints/credentials/invocation/graph wiring excluded'},'summary':{'source_file_count':len(source_files),'table_row_count':len(table_rows),'token_count':len(tokens),'json_property_count':len(json_props),'avro_field_count':len(avro_fields),'proto_field_count':len(proto_fields),'spec_prose_body_ingested':False,'actual_event_payloads_ingested':False,'broker_endpoint_or_credentials_ingested':False,'publish_subscribe_invocation_enabled':False,'mirror_graph_wiring':False},'source_files':source_files,'table_rows':table_rows[:260],'tokens':tokens[:900],'json_properties':json_props,'avro_fields':avro_fields,'proto_fields':proto_fields}
def pnix(v,indent=0):
    import json
    sp='  '*indent
    if v is None: return 'null'
    if v is True: return 'true'
    if v is False: return 'false'
    if isinstance(v,(int,float)): return json.dumps(v)
    if isinstance(v,str): return json.dumps(v,ensure_ascii=False)
    if isinstance(v,list):
        if not v: return '[ ]'
        return '[\n'+''.join(sp+'  '+pnix(x,indent+1)+'\n' for x in v)+sp+']'
    if isinstance(v,dict):
        if not v: return '{ }'
        return '{\n'+'\n'.join(sp+'  '+json.dumps(str(k),ensure_ascii=False)+' = '+pnix(v[k],indent+1)+';' for k in sorted(v))+'\n'+sp+'}'
    return json.dumps(str(v),ensure_ascii=False)
content='# stdlib/lib/corpus/cloudevents-specification.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-code-cloudevents-specification.sh && scripts/gen-code-cloudevents-specification.sh\n'
content+='# 범위: CloudEvents official 구조 메타데이터만. prose/payload/endpoints/invocation/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: files={len(source_files)} table_rows={min(len(table_rows),260)}/{len(table_rows)} tokens={min(len(tokens),900)}/{len(tokens)} json_props={len(json_props)} avro_fields={len(avro_fields)} proto_fields={len(proto_fields)} bytes={len(content.encode())}')
PY
