#!/usr/bin/env bash
# NASA DONKI raw endpoint JSON -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${NASA_DONKI_SRC:-$ROOT/ingest/science/nasa-donki}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/nasa-donki.generated.px}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then
  echo "missing NASA DONKI snapshot: $SRC" >&2
  echo "run scripts/update-science-nasa-donki.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" <<'PY'
import json, pathlib, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2])
receipt=json.loads((src/'source-receipt.json').read_text(encoding='utf-8'))
time_keys=['startTime','beginTime','peakTime','endTime','eventTime','submissionTime','messageIssueTime']
id_keys=['activityID','flrID','gstID','ipsID','sepID','mpcID','rbeID','hssID']
events=[]
for f in receipt.get('files',[]):
    ep=f['endpoint']; data=json.loads((src/f['relative_path']).read_text(encoding='utf-8'))
    if not isinstance(data,list): continue
    for item in data[:200]:
        if not isinstance(item,dict): continue
        eid=next((item.get(k) for k in id_keys if item.get(k)), None)
        times={k:item.get(k) for k in time_keys if item.get(k)}
        linked=[]
        for lk in ['linkedEvents','instruments']:
            v=item.get(lk)
            if isinstance(v,list): linked.append({'field':lk,'count':len(v)})
        events.append({'endpoint':ep,'event_id':eid,'event_type':ep,'time_fields':times,'linked_object_counts':linked,'accepted_truth':False,'experimental_notification':True,'payload_body_ingested':False,'model_payload_ingested':False,'operational_guidance_ingested':False})
obj={'schema':'science.nasa_donki_events.v1','source':{'name':'NASA DONKI space weather event API metadata','license':'NASA public data API / courtesy credit requested','source_urls':['https://api.nasa.gov/','https://api.nasa.gov/DONKI/CME'],'receipt':receipt,'generator':'scripts/gen-science-nasa-donki.sh','scope':'bounded event reference metadata only; experimental accepted_truth=false; notification prose/model payloads/mitigation guidance/graph wiring excluded'},'summary':{'event_count':len(events),'endpoint_count':len(receipt.get('files',[])),'failure_count':len(receipt.get('failures',[])),'experimental_accepted_truth_false':True,'notification_body_prose_ingested':False,'model_or_time_series_payload_ingested':False,'operational_mitigation_guidance_ingested':False,'official_warning_replacement':False,'mirror_graph_wiring':False},'events':events}
def pnix(v, indent=0):
    import json
    sp='  '*indent
    if v is None: return 'null'
    if v is True: return 'true'
    if v is False: return 'false'
    if isinstance(v,(int,float)): return json.dumps(v)
    if isinstance(v,str): return json.dumps(v, ensure_ascii=False)
    if isinstance(v,list):
        if not v: return '[ ]'
        return '[\n' + ''.join(sp+'  '+pnix(x, indent+1)+'\n' for x in v) + sp + ']'
    if isinstance(v,dict):
        if not v: return '{ }'
        return '{\n' + '\n'.join(sp+'  '+json.dumps(str(k), ensure_ascii=False)+' = '+pnix(v[k], indent+1)+';' for k in sorted(v)) + '\n' + sp + '}'
    return json.dumps(str(v), ensure_ascii=False)
content='# stdlib/lib/corpus/nasa-donki.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-science-nasa-donki.sh && scripts/gen-science-nasa-donki.sh\n'
content+='# 범위: NASA DONKI bounded event refs only. prose/model/time-series/mitigation/warning replacement/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: events={len(events)} bytes={len(content.encode())}')
PY
