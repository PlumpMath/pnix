#!/usr/bin/env bash
# OpenAlex API count/schema-key manifest -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${OPENALEX_SRC:-$ROOT/ingest/scholarly/openalex-api-manifest}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/openalex-api-manifest.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing OpenAlex API manifest snapshot: $SRC" >&2
  echo "run scripts/update-scholarly-openalex-api-manifest.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import json, pathlib, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
forbidden={'abstract_inverted_index','display_name','title','full_name','raw_author_names','referenced_works','related_works','authorships','locations','primary_location','best_oa_location','content_urls','homepage_url','image_url','image_thumbnail_url','geo'}
endpoints=[]; forbidden_hits=[]
for r in receipt.get('endpoint_rows') or []:
    keys=sorted(r.get('result_keys') or [])
    hits=sorted(set(keys)&forbidden)
    if hits: forbidden_hits.append({'endpoint':r.get('endpoint'),'keys_present_in_source_sample_but_values_excluded':hits})
    endpoints.append({'endpoint':r.get('endpoint'),'url':r.get('url'),'http_status':r.get('http_status'),'content_type':r.get('content_type'),'api_count':r.get('api_count'),'db_response_time_ms':r.get('db_response_time_ms'),'result_count_downloaded_for_key_discovery':r.get('result_count_downloaded'),'field_keys':keys})
source_files=[{'source_path':f.get('source_path'),'sha256':f.get('sha256'),'size_bytes':f.get('size_bytes'),'role':f.get('role'),'endpoint':f.get('endpoint')} for f in receipt.get('files') or []]
obj={'schema':'scholarly.openalex.api_manifest.v1','source':{'name':'OpenAlex API endpoint count/schema-key manifest','license':'CC0-1.0','source_urls':['https://developers.openalex.org/','https://github.com/ourresearch/openalex-docs/blob/main/license.md','https://developers.openalex.org/download/download-to-machine','https://developers.openalex.org/api-reference/works/get-a-single-work','https://api.openalex.org/'],'receipt_summary':{'schema':receipt.get('schema'),'source':receipt.get('source'),'retrieved_at':receipt.get('retrieved_at'),'license':receipt.get('license'),'scope':receipt.get('scope'),'base_url':receipt.get('base_url'),'mailto_used':receipt.get('mailto_used')},'generator':'scripts/gen-scholarly-openalex-api-manifest.sh','scope':'endpoint counts and field-key inventory only; API item values and snapshot payloads excluded'},'summary':{'endpoint_count':len(endpoints),'works_count':next((x.get('api_count') for x in endpoints if x.get('endpoint')=='works'),None),'record_payload_values_ingested':False,'api_item_values_ingested':False,'abstract_inverted_index_ingested':False,'titles_abstracts_person_values_ingested':False,'citation_edges_ingested':False,'snapshot_payloads_ingested':False,'linked_payloads_ingested':False,'profiling_or_ranking_ingested':False,'mirror_graph_wiring':False},'source_files':source_files,'endpoints':endpoints,'excluded_source_fields_observed':forbidden_hits,'documentation_refs':[{'name':'OpenAlex license','url':'https://github.com/ourresearch/openalex-docs/blob/main/license.md'},{'name':'Snapshot download docs','url':'https://developers.openalex.org/download/download-to-machine'},{'name':'Works object abstract warning','url':'https://developers.openalex.org/api-reference/works/get-a-single-work'}]}
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
content='# stdlib/lib/corpus/openalex-api-manifest.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-scholarly-openalex-api-manifest.sh && scripts/gen-scholarly-openalex-api-manifest.sh\n'
content+='# 범위: OpenAlex endpoint counts/field keys only. API values/abstract_inverted_index/snapshot/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: endpoints={len(endpoints)} bytes={len(content.encode())}')
PY
