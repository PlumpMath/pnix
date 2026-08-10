#!/usr/bin/env bash
# Croissant implementation schema snapshot -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${CROISSANT_SCHEMA_SRC:-$ROOT/ingest/code/croissant-schema}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/croissant-schema.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing Croissant schema snapshot: $SRC" >&2
  echo "run scripts/update-code-croissant-schema.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import ast, json, pathlib, re, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
source_files=[]; constants=[]; classes=[]; dataclass_fields=[]; enum_members=[]; functions=[]
CONST_RE=re.compile(r'^(ML_COMMONS|SCHEMA_ORG|RDF|RDFS|DCAT|BASE_IRI|NAME_REGEX|MATCHING_TYPES|DATASETS_FOLDER|TEST_DATASETS_FOLDER)')
def unparse(x):
    try: return ast.unparse(x)
    except Exception: return None
for f in receipt.get('files',[]):
    if f.get('role')!='implementation_schema_source': continue
    path=f['source_path']; p=src/f['relative_path']
    source_files.append({'source_path':path,'sha256':f['sha256'],'size_bytes':f['size_bytes']})
    tree=ast.parse(p.read_text(encoding='utf-8',errors='replace'))
    for node in tree.body:
        if isinstance(node,(ast.Assign,ast.AnnAssign)):
            targets=[]
            if isinstance(node,ast.Assign): targets=node.targets
            else: targets=[node.target]
            for t in targets:
                if isinstance(t,ast.Name) and CONST_RE.match(t.id):
                    value_kind=type(getattr(node,'value',None)).__name__ if getattr(node,'value',None) is not None else None
                    value=None
                    val=getattr(node,'value',None)
                    if isinstance(val,ast.Constant) and isinstance(val.value,(str,int,float,bool)):
                        value=val.value
                    elif isinstance(val,ast.Call):
                        value=unparse(val.func)
                    elif isinstance(val,ast.Lambda):
                        value=unparse(val.body)
                    constants.append({'source_path':path,'name':t.id,'value_kind':value_kind,'value':value})
        if isinstance(node,ast.FunctionDef):
            if not node.name.startswith('_'):
                functions.append({'source_path':path,'function':node.name,'arg_count':len(node.args.args)})
        if isinstance(node,ast.ClassDef):
            bases=[unparse(b) for b in node.bases]
            classes.append({'source_path':path,'class':node.name,'bases':[b for b in bases if b]})
            for item in node.body:
                if isinstance(item,ast.Assign):
                    for t in item.targets:
                        if isinstance(t,ast.Name) and t.id.isupper():
                            val=item.value.value if isinstance(item.value,ast.Constant) and isinstance(item.value.value,(str,int,float,bool)) else None
                            enum_members.append({'source_path':path,'class':node.name,'member':t.id,'value':val})
                if isinstance(item,ast.AnnAssign) and isinstance(item.target,ast.Name):
                    field=item.target.id
                    annotation=unparse(item.annotation)
                    value_call=None; metadata_keys=[]
                    if isinstance(item.value,ast.Call):
                        value_call=unparse(item.value.func)
                        for kw in item.value.keywords:
                            metadata_keys.append(kw.arg)
                    dataclass_fields.append({'source_path':path,'class':node.name,'field':field,'annotation':annotation,'factory':value_call,'metadata_keys':[k for k in metadata_keys if k]})
                if isinstance(item,ast.Assign):
                    for t in item.targets:
                        if isinstance(t,ast.Attribute) and isinstance(t.value,ast.Name) and t.value.id=='self':
                            dataclass_fields.append({'source_path':path,'class':node.name,'field':t.attr,'annotation':None,'factory':'self_assign','metadata_keys':[ ]})
# de-dupe
for arr,keyfn in [(constants,lambda x:(x['source_path'],x['name'])),(classes,lambda x:(x['source_path'],x['class'])),(dataclass_fields,lambda x:(x['source_path'],x['class'],x['field'],x.get('factory'))),(enum_members,lambda x:(x['source_path'],x['class'],x['member'])),(functions,lambda x:(x['source_path'],x['function']))]:
    seen=set(); outarr=[]
    for row in arr:
        k=keyfn(row)
        if k not in seen:
            seen.add(k); outarr.append(row)
    arr[:]=outarr
obj={'schema':'ml.croissant.schema.v1','source':{'name':'MLCommons Croissant mlcroissant implementation schema metadata','license':'Apache-2.0 implementation; CC-BY-ND spec prose excluded','source_urls':['https://github.com/mlcommons/croissant','https://github.com/mlcommons/croissant/tree/v1.1.0'],'receipt':receipt,'generator':'scripts/gen-code-croissant-schema.sh','scope':'implementation schema structure only; CC-BY-ND spec prose/docs/examples/dataset metadata/payloads/credentials/execution/graph wiring excluded'},'summary':{'source_file_count':len(source_files),'constant_count':len(constants),'class_count':len(classes),'dataclass_field_count':len(dataclass_fields),'enum_member_count':len(enum_members),'function_count':len(functions),'cc_by_nd_spec_body_ingested':False,'readme_or_tutorial_prose_ingested':False,'examples_or_dataset_metadata_ingested':False,'dataset_payloads_ingested':False,'credentials_ingested':False,'execution_or_invocation_enabled':False,'mirror_graph_wiring':False},'source_files':source_files,'constants':constants[:400],'classes':classes,'dataclass_fields':dataclass_fields[:500],'enum_members':enum_members[:300],'functions':functions[:200]}
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
content='# stdlib/lib/corpus/croissant-schema.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-code-croissant-schema.sh && scripts/gen-code-croissant-schema.sh\n'
content+='# 범위: Croissant Apache 구현 구조 메타데이터만. CC-BY-ND spec/docs/examples/datasets/execution/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: source_files={len(source_files)} constants={min(len(constants),400)}/{len(constants)} classes={len(classes)} dataclass_fields={min(len(dataclass_fields),500)}/{len(dataclass_fields)} enum_members={min(len(enum_members),300)}/{len(enum_members)} bytes={len(content.encode())}')
PY
