#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${FHIR_SCHEMA_SRC:-$ROOT/ingest/health/hl7-fhir-schema-catalog}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/hl7-fhir-schema-catalog.generated.px}"
LIMIT="${FHIR_SCHEMA_TERM_LIMIT:-900}"
if [[ ! -f "$SRC/source-receipt.json" ]]; then echo "missing FHIR schema snapshot: $SRC" >&2; exit 2; fi
python3 - "$SRC" "$OUT" "$LIMIT" <<'PY'
import json, pathlib, sys
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2]); limit=int(sys.argv[3])
receipt=json.loads((src/'source-receipt.json').read_text())
resources=[]; props=[]; refs=[]; required=[]
for f in receipt.get('files',[]):
    p=src/f['relative_path']
    try: data=json.loads(p.read_text(encoding='utf-8'))
    except Exception: continue
    defs=data.get('definitions') if isinstance(data,dict) else {}
    if not isinstance(defs,dict): defs={}
    for name,node in sorted(defs.items()):
        if not isinstance(node,dict): continue
        resources.append({'version':f['version'],'name':name,'kind':node.get('type'),'additional_properties':node.get('additionalProperties')})
        for r in node.get('required') or []:
            required.append({'version':f['version'],'owner':name,'field':r})
        ps=node.get('properties') if isinstance(node.get('properties'),dict) else {}
        for pn,pv in sorted(ps.items()):
            if not isinstance(pv,dict): continue
            props.append({'version':f['version'],'owner':name,'property':pn,'type':pv.get('type'),'ref':pv.get('$ref'),'items_ref':(pv.get('items') or {}).get('$ref') if isinstance(pv.get('items'),dict) else None})
            if pv.get('$ref'): refs.append({'version':f['version'],'owner':name,'property':pn,'ref':pv.get('$ref')})
resources_total=len(resources); props_total=len(props); refs_total=len(refs)
obj={'schema':'health.hl7_fhir.schema_catalog.v1','source':{'name':'HL7 FHIR JSON Schema catalog','license':'HL7 FHIR specification terms / CC0-style public specification content','source_urls':['https://hl7.org/fhir/R4/fhir.schema.json.zip','https://hl7.org/fhir/R5/fhir.schema.json.zip'],'receipt':receipt,'generator':'scripts/gen-health-hl7-fhir-schema-catalog.sh','scope':'JSON Schema structural metadata only; patient/resource payloads/examples/terminology/prose/medical advice/runtime exchange/graph wiring excluded'},'summary':{'schema_file_count':len(receipt.get('files') or []),'definition_count_total':resources_total,'property_count_total':props_total,'ref_count_total':refs_total,'definition_count_stored':min(resources_total,limit),'property_count_stored':min(props_total,limit),'ref_count_stored':min(refs_total,limit),'patient_payloads_ingested':False,'examples_ingested':False,'terminology_payloads_ingested':False,'clinical_narrative_ingested':False,'medical_advice_enabled':False,'runtime_validation_or_exchange_enabled':False,'mirror_graph_wiring':False},'definitions':resources[:limit],'properties':props[:limit],'refs':refs[:limit],'required':required[:limit]}
def pnix(v, indent=0):
    sp='  '*indent
    if v is None: return 'null'
    if v is True: return 'true'
    if v is False: return 'false'
    if isinstance(v,(int,float)): return json.dumps(v)
    if isinstance(v,str): return json.dumps(v, ensure_ascii=False)
    if isinstance(v,list): return '[ ]' if not v else '[\n'+''.join(sp+'  '+pnix(x,indent+1)+'\n' for x in v)+sp+']'
    if isinstance(v,dict): return '{ }' if not v else '{\n'+'\n'.join(sp+'  '+json.dumps(str(k),ensure_ascii=False)+' = '+pnix(v[k],indent+1)+';' for k in sorted(v))+'\n'+sp+'}'
    return json.dumps(str(v), ensure_ascii=False)
content='# stdlib/lib/corpus/hl7-fhir-schema-catalog.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-health-hl7-fhir-schema-catalog.sh && scripts/gen-health-hl7-fhir-schema-catalog.sh\n'
content+='# 범위: FHIR JSON Schema structure only. patient payloads/examples/prose/medical advice/execution/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True); out.write_text(content,encoding='utf-8')
print(f'generated {out}: defs={resources_total} props={props_total} refs={refs_total} bytes={len(content.encode())}')
PY
