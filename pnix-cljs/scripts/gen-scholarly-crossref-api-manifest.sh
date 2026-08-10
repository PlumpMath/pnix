#!/usr/bin/env bash
# Crossref REST API endpoint manifest -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${CROSSREF_SRC:-$ROOT/ingest/scholarly/crossref-api-manifest}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/crossref-api-manifest.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing Crossref API manifest snapshot: $SRC" >&2
  echo "run scripts/update-scholarly-crossref-api-manifest.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import json, pathlib, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
endpoints=[]
for r in receipt.get('endpoint_rows') or []:
    endpoints.append({'endpoint':r.get('endpoint'),'url':r.get('url'),'http_status':r.get('http_status'),'content_type':r.get('content_type'),'api_status':r.get('api_status'),'total_results':r.get('total_results'),'items_per_page':r.get('items_per_page'),'items_count':r.get('items_count')})
source_files=[{'source_path':f.get('source_path'),'sha256':f.get('sha256'),'size_bytes':f.get('size_bytes'),'role':f.get('role'),'endpoint':f.get('endpoint')} for f in receipt.get('files') or []]
obj={'schema':'scholarly.crossref.api_manifest.v1','source':{'name':'Crossref REST API endpoint manifest','license':'Crossref Free Services metadata rights / no ownership claims','source_urls':['https://www.crossref.org/documentation/retrieve-metadata/rest-api/','https://github.com/Crossref/rest-api-doc','https://www.crossref.org/documentation/retrieve-metadata/bulk-downloads/','https://www.crossref.org/blog/2026-public-data-file-now-available/','https://api.crossref.org/'],'receipt_summary':{'schema':receipt.get('schema'),'source':receipt.get('source'),'retrieved_at':receipt.get('retrieved_at'),'license':receipt.get('license'),'scope':receipt.get('scope'),'base_url':receipt.get('base_url'),'mailto_used':receipt.get('mailto_used')},'generator':'scripts/gen-scholarly-crossref-api-manifest.sh','scope':'official endpoint manifest and rows=0 counts only; DOI/API record payloads excluded'},'summary':{'endpoint_count':len(endpoints),'rows_zero_only':True,'total_works':next((x.get('total_results') for x in endpoints if x.get('endpoint')=='works'),None),'doi_record_payloads_ingested':False,'api_items_ingested':False,'titles_abstracts_person_values_ingested':False,'references_or_work_license_arrays_ingested':False,'public_data_file_payloads_ingested':False,'metadata_plus_snapshots_ingested':False,'linked_payloads_ingested':False,'profiling_or_ranking_ingested':False,'mirror_graph_wiring':False},'source_files':source_files,'endpoints':endpoints,'documentation_refs':[{'name':'REST API documentation','url':'https://www.crossref.org/documentation/retrieve-metadata/rest-api/'},{'name':'Metadata license text','url':'https://github.com/Crossref/rest-api-doc#metadata-license'},{'name':'Bulk downloads and snapshots','url':'https://www.crossref.org/documentation/retrieve-metadata/bulk-downloads/'},{'name':'2026 public data file announcement','url':'https://www.crossref.org/blog/2026-public-data-file-now-available/'}]}
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
content='# stdlib/lib/corpus/crossref-api-manifest.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-scholarly-crossref-api-manifest.sh && scripts/gen-scholarly-crossref-api-manifest.sh\n'
content+='# 범위: Crossref REST API endpoint manifest rows=0 only. DOI records/prose/person/API payload/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: endpoints={len(endpoints)} bytes={len(content.encode())}')
PY
