#!/usr/bin/env bash
# ROR authority snapshot -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${ROR_SRC:-$ROOT/ingest/scholarly/ror-authority}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/ror-authority.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing ROR snapshot: $SRC" >&2
  echo "run scripts/update-scholarly-ror-authority.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import json, pathlib, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
source_files=[]; orgs=[]; names=[]; relationships=[]; external_ids=[]; countries=[]; type_counts={}; status_counts={}
for f in receipt.get('files',[]):
    if f.get('role')!='ror_record_json': continue
    path=f['source_path']; p=src/f['relative_path']
    source_files.append({'source_path':path,'sha256':f['sha256'],'size_bytes':f['size_bytes']})
    data=json.loads(p.read_text(encoding='utf-8'))
    rid=data.get('id')
    name=None
    for n in data.get('names') or []:
        if 'ror_display' in (n.get('types') or []): name=n.get('value')
    if name is None:
        name=data.get('name') or next((n.get('value') for n in (data.get('names') or []) if n.get('value')), None)
    country=data.get('country') or {}
    locs=data.get('locations') or []
    country_code=country.get('country_code') or next(((l.get('geonames_details') or {}).get('country_code') for l in locs if isinstance(l,dict)), None)
    country_name=country.get('country_name') or next(((l.get('geonames_details') or {}).get('country_name') for l in locs if isinstance(l,dict)), None)
    types=data.get('types') or []
    status=data.get('status')
    for t in types: type_counts[t]=type_counts.get(t,0)+1
    status_counts[status]=status_counts.get(status,0)+1
    orgs.append({'ror_id':rid,'record_key':rid.rsplit('/',1)[-1] if isinstance(rid,str) else None,'display_name':name,'status':status,'types':types[:8],'established':data.get('established'),'country_code':country_code,'country_name':country_name,'has_links':bool(data.get('links')),'link_count':len(data.get('links') or []),'location_count':len(locs),'external_id_count':len(data.get('external_ids') or [])})
    for n in (data.get('names') or [])[:12]:
        val=n.get('value')
        if val: names.append({'ror_id':rid,'value':val,'lang':n.get('lang'),'types':(n.get('types') or [])[:8]})
    for rel in (data.get('relationships') or [])[:20]:
        relationships.append({'ror_id':rid,'type':rel.get('type'),'target_id':rel.get('id')})
    for ex in (data.get('external_ids') or [])[:20]:
        external_ids.append({'ror_id':rid,'type':ex.get('type'),'preferred':ex.get('preferred'),'all_count':len(ex.get('all') or [])})
    if country_code or country_name:
        countries.append({'ror_id':rid,'country_code':country_code,'country_name':country_name})
summary_types=[{'type':k,'count':v} for k,v in sorted(type_counts.items())]
summary_status=[{'status':k,'count':v} for k,v in sorted(status_counts.items(), key=lambda kv: str(kv[0]))]
receipt_summary={'schema':receipt.get('schema'),'source':receipt.get('source'),'ref':receipt.get('ref'),'version_dir':receipt.get('version_dir'),'retrieved_at':receipt.get('retrieved_at'),'source_urls':receipt.get('source_urls'),'license':receipt.get('license'),'scope':receipt.get('scope'),'record_file_count_total_in_version_dir':receipt.get('record_file_count_total_in_version_dir'),'record_file_count_stored':receipt.get('record_file_count_stored')}
obj={'schema':'scholarly.ror.authority.v1','source':{'name':'ROR official organization authority records','license':'MIT repository license / open ROR authority data','source_urls':['https://github.com/ror-community/ror-records','https://ror.org/'],'receipt_summary':receipt_summary,'generator':'scripts/gen-scholarly-ror-authority.sh','scope':'bounded organization authority rows only; coordinates/address lines/domains/URL payloads/email/IP/live API harvest/web page bodies/graph wiring excluded'},'summary':{'record_file_count':len(source_files),'org_count':len(orgs),'name_row_count':len(names),'relationship_row_count':len(relationships),'external_id_row_count':len(external_ids),'country_row_count':len(countries),'address_lines_ingested':False,'coordinates_ingested':False,'domains_and_links_payload_ingested':False,'email_ip_fields_ingested':False,'web_page_bodies_ingested':False,'live_api_harvest_ingested':False,'mirror_graph_wiring':False},'source_files':source_files,'organizations':orgs,'names':names[:2000],'relationships':relationships[:2000],'external_ids':external_ids[:1500],'countries':countries,'type_counts':summary_types,'status_counts':summary_status}
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
content='# stdlib/lib/corpus/ror-authority.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-scholarly-ror-authority.sh && scripts/gen-scholarly-ror-authority.sh\n'
content+='# 범위: ROR organization authority 구조 메타데이터만. coordinates/address/domains/URL payloads/email/IP/live harvest/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: orgs={len(orgs)} names={min(len(names),2000)}/{len(names)} rels={min(len(relationships),2000)}/{len(relationships)} external_ids={min(len(external_ids),1500)}/{len(external_ids)} bytes={len(content.encode())}')
PY
