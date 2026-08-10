#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/ingest/bio/catalogue-of-life-api-catalog"
OUT="$ROOT/stdlib/lib/corpus/catalogue-of-life-api-catalog.generated.px"
SCHEMA_LIMIT="${COL_OPENAPI_SCHEMA_LIMIT:-250}"
PATH_LIMIT="${COL_OPENAPI_PATH_LIMIT:-220}"
python3 - "$SRC" "$OUT" "$SCHEMA_LIMIT" "$PATH_LIMIT" <<'PY'
import pathlib, sys, json, hashlib, re
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2]); schema_limit=int(sys.argv[3]); path_limit=int(sys.argv[4])
def esc(s): return '"'+str(s).replace('\\','\\\\').replace('"','\\"').replace('$','DOLLAR_SIGN').replace('\n',' ').replace('\r','')+'"'
def lit(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,list): return '[ ' + ' '.join(lit(x) for x in v) + ' ]'
    if isinstance(v,dict): return '{ ' + ' '.join(f'{k} = {lit(val)};' for k,val in v.items()) + ' }'
    return esc('' if v is None else v)
def load(name, default):
    p=src/name
    return json.loads(p.read_text()) if p.exists() else default
def sig_file(name):
    p=src/name
    return {'path':name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size} if p.exists() else {'path':name,'sha256':'','bytes':0}
def clean_text_hash(s):
    s='' if s is None else re.sub(r'\s+',' ',str(s)).strip()
    return {'present': bool(s), 'sha256': hashlib.sha256(s.encode()).hexdigest() if s else '', 'len': len(s)}
def dataset_compact(d):
    pub=d.get('publisher') or {}
    out={'key':d.get('key',0),'title':d.get('title',''),'label':d.get('label',''),'type':d.get('type',''),'origin':d.get('origin',''),'alias':d.get('alias',''),'version':d.get('version',''),'issued':d.get('issued',''),'size':d.get('size',0),'license':d.get('license',''),'doi':d.get('doi',''),'version_doi':d.get('versionDoi',''),'private':bool(d.get('private',False)),'publisher':{'name':pub.get('name',''),'country':pub.get('country',''),'organisation':pub.get('organisation','')},'taxonomic_group_scope':d.get('taxonomicGroupScope',[])[:80]}
    if d.get('citation'): out['citation_html_hash']=clean_text_hash(d.get('citation'))
    return out
def schema_prop_type(p):
    if not isinstance(p,dict): return ''
    if '$ref' in p: return p['$ref'].split('/')[-1]
    t=p.get('type') or (p.get('types') or [''])[0]
    if isinstance(t,list): t=','.join(map(str,t))
    if p.get('format'): t=f'{t}:{p.get("format")}'
    if p.get('enum'): t=f'{t}:enum({len(p.get("enum") or [])})'
    return str(t)
datasets=load('datasets.json',{}); colsets=load('col-datasets.json',{}); ranks=load('ranks.json',[]); openapi=load('openapi.json',{})
dataset_records=[dataset_compact(x) for x in (datasets.get('result') or [])]
col_dataset_records=[dataset_compact(x) for x in (colsets.get('result') or [])]
rank_records=[]
for r in ranks:
    rank_records.append({k:r.get(k,'') if not isinstance(r.get(k),bool) else bool(r.get(k)) for k in ['name','plural','marker','majorRank','code','restrictedToCode','linnean','familyGroup','genusGroup','infraspecific','suprageneric','supraspecific','legacy','uncomparable','ambiguousMarker'] if k in r})
paths=[]
for path,methods in sorted((openapi.get('paths') or {}).items()):
    if len(paths)>=path_limit: break
    for method,op in sorted((methods or {}).items()):
        if method.lower() not in ['get','post','put','delete','patch']: continue
        params=[]
        for p in op.get('parameters') or []:
            params.append({'name':p.get('name',''),'param_in':p.get('in',''),'required':bool(p.get('required',False)),'schema':schema_prop_type(p.get('schema') or {})})
        paths.append({'path':path,'method':method.upper(),'operation_id':op.get('operationId',''),'tags':op.get('tags',[])[:8],'parameters':params[:40]})
        if len(paths)>=path_limit: break
schemas=[]
for name,schema in sorted(((openapi.get('components') or {}).get('schemas') or {}).items()):
    if len(schemas)>=schema_limit: break
    props=[]
    for pn,pv in sorted((schema.get('properties') or {}).items()):
        props.append({'name':pn,'type':schema_prop_type(pv)})
    schemas.append({'name':name,'type':schema.get('type',''),'required':schema.get('required',[])[:80],'properties':props[:120]})
files=[sig_file(n) for n in ['source-manifest.json','datasets.json','col-datasets.json','ranks.json','openapi.json']]
manifest=load('source-manifest.json',{})
obj={'schema':'bio.catalogue_of_life.api_catalog.v1','source':'Catalogue of Life / ChecklistBank API catalog metadata','license':'COL/ChecklistBank API catalog metadata; dataset payload licenses not ingested','summary':{'dataset_records':len(dataset_records),'col_dataset_records':len(col_dataset_records),'rank_records':len(rank_records),'openapi_paths':len(paths),'openapi_schemas':len(schemas),'openapi_schema_limit':schema_limit,'openapi_path_limit':path_limit},'policy':'API/dataset/rank/OpenAPI catalog metadata only. Excludes taxon/name payload records, full checklist dumps, citation HTML values, prose descriptions/examples, vernacular/distribution/reference payloads, credentials, biodiversity management advice, and graph/mirror wiring.','manifest':manifest,'files':files,'datasets':dataset_records,'col_datasets':col_dataset_records,'ranks':rank_records,'openapi':{'title':((openapi.get('info') or {}).get('title','')),'version':((openapi.get('info') or {}).get('version','')),'paths':paths,'schemas':schemas}}
out.write_text('# GENERATED by scripts/gen-bio-catalogue-of-life-api-catalog.sh. Do not edit. Gitignored.\n# Source: COL/ChecklistBank API catalog metadata only; taxon payloads/prose excluded.\n'+lit(obj)+'\n',encoding='utf-8')
print(f'generated {out}: datasets={len(dataset_records)} col_datasets={len(col_dataset_records)} ranks={len(rank_records)} paths={len(paths)} schemas={len(schemas)} bytes={out.stat().st_size}')
PY
