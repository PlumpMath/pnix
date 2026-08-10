#!/usr/bin/env bash
# MLflow registry snapshot -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${MLFLOW_REGISTRY_SRC:-$ROOT/ingest/code/mlflow-registry}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/mlflow-registry.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing MLflow registry snapshot: $SRC" >&2
  echo "run scripts/update-code-mlflow-registry.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import ast, json, pathlib, re, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
source_files=[]; proto_files=[]; imports=[]; messages=[]; enums=[]; fields=[]; rpcs=[]; entity_classes=[]; entity_attrs=[]; enum_tokens=[]
STRIP_COMMENTS=re.compile(r'//.*?$|/\*.*?\*/', re.S|re.M)
for f in receipt.get('files',[]):
    if f.get('role') not in ('proto_schema','registry_entity_source'): continue
    path=f['source_path']; rel=f['relative_path']; p=src/rel
    source_files.append({'source_path':path,'role':f['role'],'sha256':f['sha256'],'size_bytes':f['size_bytes']})
    if f['role']=='proto_schema':
        text=p.read_text(encoding='utf-8',errors='replace')
        no=STRIP_COMMENTS.sub('',text)
        package=(re.search(r'\bpackage\s+([A-Za-z0-9_.]+)\s*;',no) or [None,None])[1]
        syntax=(re.search(r'\bsyntax\s*=\s*"([^"]+)"\s*;',no) or [None,None])[1]
        proto_files.append({'source_path':path,'package':package,'syntax':syntax})
        for m in re.finditer(r'\bimport\s+"([^"]+)"\s*;',no): imports.append({'source_path':path,'import':m.group(1)})
        for m in re.finditer(r'\bmessage\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{',no): messages.append({'source_path':path,'package':package,'message':m.group(1)})
        for m in re.finditer(r'\benum\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{',no): enums.append({'source_path':path,'package':package,'enum':m.group(1)})
        for em in re.finditer(r'\benum\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{(.*?)\n\}',no,re.S):
            en=em.group(1)
            for tm in re.finditer(r'(?m)^\s*([A-Z][A-Z0-9_]*)\s*=\s*([0-9]+)\s*;',em.group(2)):
                enum_tokens.append({'source_path':path,'enum':en,'name':tm.group(1),'number':int(tm.group(2))})
        for m in re.finditer(r'\brpc\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]+)\)\s+returns\s+\(([^)]+)\)',no):
            name=m.group(1)
            if any(x in name.lower() for x in ['registeredmodel','modelversion','transition','webhook','prompt']):
                rpcs.append({'source_path':path,'rpc':name,'input_type':m.group(2).strip(),'output_type':m.group(3).strip()})
        field_re=re.compile(r'(?m)^\s*(optional|required|repeated)?\s*([A-Za-z_][A-Za-z0-9_.<>]*)\s+([A-Za-z_][A-Za-z0-9_]*)\s*=\s*([0-9]+)')
        for m in field_re.finditer(no):
            typ=m.group(2); name=m.group(3)
            if typ in ('option','reserved','extensions','message','enum','service','rpc','returns'): continue
            if any(k in name.lower() or k in typ.lower() for k in ['artifact','metric','param']):
                # schema row only; still do not ingest values. Keep field names, not values.
                pass
            fields.append({'source_path':path,'package':package,'label':m.group(1) or 'singular','type':typ,'name':name,'number':int(m.group(4))})
    else:
        tree=ast.parse(p.read_text(encoding='utf-8',errors='replace'))
        for node in tree.body:
            if isinstance(node,ast.ClassDef):
                entity_classes.append({'source_path':path,'class':node.name,'base_count':len(node.bases)})
                for sub in ast.walk(node):
                    if isinstance(sub,ast.FunctionDef) and sub.name=='__init__':
                        for arg in sub.args.args[1:]:
                            entity_attrs.append({'source_path':path,'class':node.name,'attr':arg.arg,'kind':'init_arg'})
                    if isinstance(sub,ast.Assign):
                        for t in sub.targets:
                            if isinstance(t,ast.Attribute) and isinstance(t.value,ast.Name) and t.value.id=='self':
                                entity_attrs.append({'source_path':path,'class':node.name,'attr':t.attr,'kind':'self_attr'})
# de-dupe attrs
seen=set(); attrs=[]
for a in entity_attrs:
    k=(a['source_path'],a['class'],a['attr'],a['kind'])
    if k not in seen:
        seen.add(k); attrs.append(a)
obj={'schema':'ml.mlflow.registry_schema.v1','source':{'name':'MLflow official model registry schema metadata','license':'Apache-2.0','source_urls':['https://github.com/mlflow/mlflow','https://github.com/mlflow/mlflow/tree/v3.14.0'],'receipt':receipt,'generator':'scripts/gen-code-mlflow-registry.sh','scope':'official registry proto/entity structure only; experiment values/tracking artifacts/model files/weights/credentials/live registry rows/invocation/graph wiring excluded'},'summary':{'source_file_count':len(source_files),'proto_file_count':len(proto_files),'import_count':len(imports),'message_count':len(messages),'enum_count':len(enums),'field_count':len(fields),'registry_rpc_count':len(rpcs),'entity_class_count':len(entity_classes),'entity_attr_count':len(attrs),'enum_token_count':len(enum_tokens),'experiment_run_values_ingested':False,'tracking_artifacts_ingested':False,'model_files_or_weights_ingested':False,'credentials_ingested':False,'live_registry_rows_ingested':False,'execution_or_invocation_enabled':False,'mirror_graph_wiring':False},'source_files':source_files,'proto_files':proto_files,'imports':imports,'messages':messages,'enums':enums,'fields':fields[:700],'registry_rpcs':rpcs[:180],'entity_classes':entity_classes,'entity_attrs':attrs[:200],'enum_tokens':enum_tokens[:300]}
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
content='# stdlib/lib/corpus/mlflow-registry.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-code-mlflow-registry.sh && scripts/gen-code-mlflow-registry.sh\n'
content+='# 범위: MLflow registry schema 구조 메타데이터만. experiment/artifacts/models/credentials/invocation/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: proto_files={len(proto_files)} messages={len(messages)} enums={len(enums)} fields={min(len(fields),700)}/{len(fields)} registry_rpcs={min(len(rpcs),180)}/{len(rpcs)} entity_attrs={min(len(attrs),200)}/{len(attrs)} bytes={len(content.encode())}')
PY
