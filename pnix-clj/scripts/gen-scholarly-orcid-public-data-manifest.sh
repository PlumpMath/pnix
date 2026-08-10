#!/usr/bin/env bash
# ORCID Public Data File manifest -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${ORCID_PUBLIC_DATA_SRC:-$ROOT/ingest/scholarly/orcid-public-data-manifest}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/orcid-public-data-manifest.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing ORCID public data manifest snapshot: $SRC" >&2
  echo "run scripts/update-scholarly-orcid-public-data-manifest.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import json, pathlib, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
archive_files=[]
for f in receipt.get('archive_files') or []:
    archive_files.append({'id':f.get('id'),'name':f.get('name'),'size_bytes':f.get('size'),'download_url':f.get('download_url'),'computed_md5':f.get('computed_md5'),'supplied_md5':f.get('supplied_md5')})
source_files=[{'source_path':f.get('source_path'),'sha256':f.get('sha256'),'size_bytes':f.get('size_bytes'),'role':f.get('role')} for f in receipt.get('files') or []]
article=receipt.get('article') or {}
obj={'schema':'scholarly.orcid.public_data_manifest.v1','source':{'name':'ORCID Public Data File Figshare article/file manifest','license':'CC0-1.0','source_urls':['https://info.orcid.org/what-is-orcid/services/annual-data-files/','https://info.orcid.org/public-data-file-use-policy/','https://orcid.figshare.com/articles/dataset/ORCID_Public_Data_File_2025/30375589'],'receipt_summary':{'schema':receipt.get('schema'),'source':receipt.get('source'),'article_id':receipt.get('article_id'),'api_url':receipt.get('api_url'),'retrieved_at':receipt.get('retrieved_at'),'license':receipt.get('license'),'scope':receipt.get('scope')},'generator':'scripts/gen-scholarly-orcid-public-data-manifest.sh','scope':'official annual public data file manifest only; record tarballs and ORCID Public API harvests excluded'},'summary':{'article_title':article.get('title'),'doi':article.get('doi'),'version':article.get('version'),'published_date':article.get('published_date'),'modified_date':article.get('modified_date'),'defined_type_name':article.get('defined_type_name'),'license_name':(article.get('license') or {}).get('name') if isinstance(article.get('license'),dict) else None,'file_count':article.get('file_count'),'total_file_size_bytes':article.get('total_file_size_bytes'),'tarball_payloads_downloaded':False,'orcid_api_harvest_ingested':False,'individual_records_ingested':False,'summaries_activities_ingested':False,'biography_work_profile_values_ingested':False,'email_ip_fields_ingested':False,'linked_payloads_ingested':False,'profiling_or_ranking_ingested':False,'mirror_graph_wiring':False},'source_files':source_files,'archive_files':archive_files}
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
content='# stdlib/lib/corpus/orcid-public-data-manifest.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-scholarly-orcid-public-data-manifest.sh && scripts/gen-scholarly-orcid-public-data-manifest.sh\n'
content+='# 범위: ORCID Public Data File Figshare manifest only. record/profile/API harvest/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: archive_files={len(archive_files)} bytes={len(content.encode())}')
PY
