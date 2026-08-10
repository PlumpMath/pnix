#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IN="$ROOT/ingest/device/fda-product-classification/device-classification.json"
OUT="$ROOT/stdlib/lib/corpus/fda-product-classification.generated.px"
python3 - "$IN" "$OUT" <<'PY'
import json, sys
from pathlib import Path
j=json.load(open(sys.argv[1])); out=Path(sys.argv[2])

def esc(s): return '"'+str(s).replace('\\','\\\\').replace('"','\\"').replace('\n',' ').replace('\r',' ')+'"'
def pn(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,str): return esc(v)
    if isinstance(v,list): return '[ '+' '.join(pn(x) for x in v)+' ]'
    if isinstance(v,dict): return '{ '+' '.join(f'{k} = {pn(val)};' for k,val in v.items())+' }'
    if v is None: return 'null'
    raise TypeError(type(v))
fields=['product_code','device_name','device_class','medical_specialty','medical_specialty_description','review_panel','review_code','unclassified_reason','regulation_number','implant_flag','life_sustain_support_flag','third_party_flag','gmp_exempt_flag','summary_malfunction_reporting','submission_type_id']
records=[]
for r in j.get('results',[]):
    row={k:str(r.get(k,'')) for k in fields if r.get(k,'') not in (None,'')}
    # Explicitly drop openfda arrays and definition prose.
    records.append(row)
meta=j.get('meta',{})
seed={'schema':'device.fda_product_classification.v1','source':{'name':'openFDA Device Classification','license':'CC0-1.0 / FDA public domain unless otherwise noted','endpoint':'https://api.fda.gov/device/classification.json'},'summary':{'last_updated':meta.get('last_updated',''),'api_total':meta.get('results',{}).get('total',0),'stored_count':len(records),'openfda_identifier_arrays_ingested':False,'definition_prose_ingested':False,'adverse_event_reports_ingested':False,'recall_narratives_ingested':False,'medical_advice_ingested':False,'mirror_graph_wiring':False},'classifications':records}
out.write_text(pn(seed)+'\n')
print(f'generated {out}: records={len(records)} total={meta.get("results",{}).get("total",0)}')
PY
