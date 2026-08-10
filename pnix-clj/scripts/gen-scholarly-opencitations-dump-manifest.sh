#!/usr/bin/env bash
# OpenCitations dump manifests -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${OPENCITATIONS_SRC:-$ROOT/ingest/scholarly/opencitations-dump-manifest}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/opencitations-dump-manifest.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing OpenCitations dump manifest snapshot: $SRC" >&2
  echo "run scripts/update-scholarly-opencitations-dump-manifest.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import json, pathlib, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
articles=[]
for a in receipt.get('articles') or []:
    articles.append({'article_id':a.get('article_id'),'title':a.get('title'),'doi':a.get('doi'),'version':a.get('version'),'published_date':a.get('published_date'),'modified_date':a.get('modified_date'),'defined_type_name':a.get('defined_type_name'),'license_name':(a.get('license') or {}).get('name') if isinstance(a.get('license'),dict) else None,'license_url':(a.get('license') or {}).get('url') if isinstance(a.get('license'),dict) else None,'file_count':a.get('file_count'),'total_file_size_bytes':a.get('total_file_size_bytes'),'figshare_url':a.get('figshare_url'),'url_public_api':a.get('url_public_api')})
archive_files=[]
for f in receipt.get('archive_files') or []:
    archive_files.append({'article_id':f.get('article_id'),'id':f.get('id'),'name':f.get('name'),'size_bytes':f.get('size'),'download_url':f.get('download_url'),'computed_md5':f.get('computed_md5'),'supplied_md5':f.get('supplied_md5')})
source_files=[{'source_path':f.get('source_path'),'sha256':f.get('sha256'),'size_bytes':f.get('size_bytes'),'role':f.get('role'),'article_id':f.get('article_id')} for f in receipt.get('files') or []]
obj={'schema':'scholarly.opencitations.dump_manifest.v1','source':{'name':'OpenCitations Figshare dump article/file manifests','license':'CC0-1.0','source_urls':['https://opencitations.net/','https://download.opencitations.net/','https://figshare.com/articles/dataset/OpenCitations_Index_RDF_Data_Dump/31353691','https://figshare.com/articles/dataset/OpenCitations_Index_CSV_dataset_storing_data_source_information_about_all_the_citation_data/28677293','https://figshare.com/articles/dataset/OpenCitations_Meta_CSV_dataset_of_all_bibliographic_metadata/21747461'],'receipt_summary':{'schema':receipt.get('schema'),'source':receipt.get('source'),'article_ids':receipt.get('article_ids'),'retrieved_at':receipt.get('retrieved_at'),'license':receipt.get('license'),'scope':receipt.get('scope')},'generator':'scripts/gen-scholarly-opencitations-dump-manifest.sh','scope':'official dump manifests only; RDF/CSV payloads and API/SPARQL harvests excluded'},'summary':{'article_count':len(articles),'archive_file_count':len(archive_files),'total_archive_file_size_bytes':sum((f.get('size_bytes') or 0) for f in archive_files),'dump_payloads_downloaded':False,'citation_edge_rows_ingested':False,'bibliographic_record_values_ingested':False,'api_sparql_harvest_ingested':False,'web_page_bodies_ingested':False,'profiling_or_ranking_ingested':False,'mirror_graph_wiring':False},'source_files':source_files,'articles':articles,'archive_files':archive_files}
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
content='# stdlib/lib/corpus/opencitations-dump-manifest.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-scholarly-opencitations-dump-manifest.sh && scripts/gen-scholarly-opencitations-dump-manifest.sh\n'
content+='# 범위: OpenCitations Figshare dump manifests only. RDF/CSV payload/API harvest/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: articles={len(articles)} archive_files={len(archive_files)} bytes={len(content.encode())}')
PY
