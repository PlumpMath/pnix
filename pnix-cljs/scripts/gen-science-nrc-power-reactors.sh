#!/usr/bin/env bash
# NRC operating power reactors workbook -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${NRC_POWER_REACTORS_SRC:-$ROOT/ingest/science/nrc-power-reactors/reactors-operating.xls}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/nrc-power-reactors.generated.px}"
RECEIPT="${NRC_POWER_REACTORS_RECEIPT:-$ROOT/ingest/science/nrc-power-reactors/source-receipt.json}"
if [[ ! -f "$SRC" ]]; then
  echo "missing NRC workbook: $SRC" >&2
  echo "run scripts/update-science-nrc-power-reactors.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import datetime as dt, hashlib, io, json, pathlib, re, sys, zipfile, xml.etree.ElementTree as ET
src=pathlib.Path(sys.argv[1]); out=pathlib.Path(sys.argv[2]); receipt_path=pathlib.Path(sys.argv[3])
raw=src.read_bytes(); receipt=json.loads(receipt_path.read_text(encoding='utf-8')) if receipt_path.exists() else {}
z=zipfile.ZipFile(io.BytesIO(raw)); ns={'a':'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
shared=[]
if 'xl/sharedStrings.xml' in z.namelist():
    root=ET.fromstring(z.read('xl/sharedStrings.xml'))
    for si in root.findall('a:si',ns): shared.append(''.join(t.text or '' for t in si.findall('.//a:t',ns)))
def colnum(cell):
    m=re.match(r'([A-Z]+)',cell or 'A'); n=0
    for ch in m.group(1): n=n*26+ord(ch)-64
    return n
def cellval(c):
    if c.attrib.get('t')=='inlineStr': return ''.join(t.text or '' for t in c.findall('.//a:t',ns)).strip()
    v=c.find('a:v',ns); txt='' if v is None else (v.text or '')
    if c.attrib.get('t')=='s' and txt:
        try: return shared[int(txt)].strip()
        except Exception: return txt.strip()
    return txt.strip()
def clean(v):
    if v is None: return None
    v=str(v).replace('\xa0',' ').strip()
    v=re.sub(r'\s+',' ',v)
    return v or None
def excel_date(v):
    v=clean(v)
    if not v: return None
    if re.match(r'^\d{1,2}/\d{1,2}/\d{4}$', v):
        m,d,y=map(int,v.split('/'))
        return f'{y:04d}-{m:02d}-{d:02d}'
    if re.match(r'^\d+(\.0)?$', v):
        n=int(float(v))
        # Excel 1900 date system with leap-year bug offset.
        base=dt.date(1899,12,30)
        return (base+dt.timedelta(days=n)).isoformat()
    return v
def state_from_location(v):
    v=clean(v)
    if not v: return None
    states=re.findall(r'\b([A-Z]{2})\b', v)
    return states[0] if states else None
sheet='xl/worksheets/sheet1.xml'
root=ET.fromstring(z.read(sheet))
rows=[]
for row in root.findall('.//a:row',ns):
    vals={colnum(c.attrib.get('r','A1')):cellval(c) for c in row.findall('a:c',ns)}
    rows.append((int(row.attrib.get('r','0')), vals))
header=[clean(rows[0][1].get(i)) or '' for i in range(1, max(rows[0][1].keys())+1)]
idx={h:i+1 for i,h in enumerate(header) if h}
def get(vals,name): return clean(vals.get(idx.get(name,-999)))
records=[]; region_counts={}; state_counts={}; reactor_type_counts={}; year_counts={}
for rn, vals in rows[1:]:
    if not get(vals,'Plant Name, Unit Number') or not get(vals,'Docket Number'): continue
    loc=get(vals,'Location')
    rec={
      'year_of_update': get(vals,'Year of Update'),
      'plant_unit_name': get(vals,'Plant Name, Unit Number'),
      'nrc_reactor_unit_web_page_label': get(vals,'NRC Reactor Unit Web Page'),
      'docket_number': get(vals,'Docket Number'),
      'license_number': get(vals,'License Number'),
      'state': state_from_location(loc),
      'nrc_region': get(vals,'NRC Region'),
      'parent_company_utility_name': get(vals,'Parent Company Utility Name'),
      'licensee': get(vals,'Licensee'),
      'reactor_and_containment_type': get(vals,'Reactor and Containment Type'),
      'nuclear_steam_system_supplier_and_design_type': get(vals,'Nuclear Steam System Supplier and Design Type'),
      'operating_license_issued': excel_date(get(vals,'Operating License Issued')),
      'operating_license_expires': excel_date(get(vals,'Operating License Expires')),
      'raw_location_text_ingested': False,
      'precise_coordinates_ingested': False,
      'capacity_or_current_status_ingested': False,
      'operational_or_security_guidance_ingested': False,
    }
    records.append(rec)
    for k,d in [('nrc_region',region_counts),('state',state_counts),('reactor_and_containment_type',reactor_type_counts),('year_of_update',year_counts)]:
        x=rec.get(k) or 'unknown'; d[x]=d.get(x,0)+1
def pairs(d,k): return [{k:x,'count':d[x]} for x in sorted(d)]
obj={
 'schema':'science.nrc_power_reactors.v1',
 'source':{
   'name':'NRC Commercial Nuclear Power Reactors – Operating Reactors dataset',
   'license':'NRC U.S. Government Work public-domain / no copyright; courtesy credit requested',
   'source_urls':['https://www.nrc.gov/reading-rm/doc-collections/datasets/index.html','https://www.nrc.gov/data/index','https://www.nrc.gov/site-help/disclaimer'],
   'receipt':receipt,
   'generator':'scripts/gen-science-nrc-power-reactors.sh',
   'scope':'bounded facility/reactor/docket/license identifier metadata only; raw location, precise coordinates, capacity/current status, event/status feeds, security/emergency/operational guidance, and graph/mirror wiring excluded'
 },
 'source_files':{'workbook_sha256':hashlib.sha256(raw).hexdigest(),'workbook_size_bytes':len(raw)},
 'summary':{
   'reactor_unit_count':len(records),
   'year_counts':pairs(year_counts,'year_of_update'),
   'state_counts':pairs(state_counts,'state'),
   'nrc_region_counts':pairs(region_counts,'nrc_region'),
   'reactor_type_counts':pairs(reactor_type_counts,'reactor_and_containment_type'),
   'raw_location_text_ingested':False,
   'precise_coordinates_ingested':False,
   'capacity_or_current_status_ingested':False,
   'reactor_status_daily_feed_ingested':False,
   'scram_event_inspection_enforcement_data_ingested':False,
   'operational_or_security_guidance_ingested':False,
   'emergency_response_instruction_ingested':False,
   'mirror_graph_wiring':False,
 },
 'reactor_units':records,
}
def pnix(v, indent=0):
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
content='# stdlib/lib/corpus/nrc-power-reactors.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-science-nrc-power-reactors.sh && scripts/gen-science-nrc-power-reactors.sh\n'
content+='# 범위: NRC reactor/facility/docket/license identifiers only. raw location/capacity/status/security/emergency/graph wiring 제외.\n'
content+=pnix(obj)+'\n'
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(content, encoding='utf-8')
print(f'generated {out}: reactor_units={len(records)} bytes={len(content.encode())}')
PY
