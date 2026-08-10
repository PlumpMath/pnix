#!/usr/bin/env bash
# EIA-860 generator workbook -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${EIA860_SRC:-$ROOT/ingest/energy/eia-860-generator-inventory/eia860.zip}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/eia-860-generator-inventory.generated.px}"
RECEIPT="$ROOT/ingest/energy/eia-860-generator-inventory/source-receipt.json"
LIMIT="${EIA860_LIMIT:-250}"
if [[ ! -f "$SRC" ]]; then
  echo "missing EIA-860 zip: $SRC" >&2
  echo "run scripts/update-energy-eia-860-generator-inventory.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" "$LIMIT" <<'PY'
import hashlib, io, json, pathlib, re, sys, zipfile, xml.etree.ElementTree as ET
src, out, receipt_path = map(pathlib.Path, sys.argv[1:4]); limit=int(sys.argv[4])
raw=src.read_bytes()
try: receipt=json.loads(receipt_path.read_text(encoding='utf-8'))
except Exception: receipt={}
z=zipfile.ZipFile(io.BytesIO(raw))
gen_names=[n for n in z.namelist() if re.search(r'3_1_.*Generator.*\.xlsx$', n, flags=re.I)]
if not gen_names:
    gen_names=[n for n in z.namelist() if 'Generator' in n and n.lower().endswith('.xlsx')]
if not gen_names:
    raise SystemExit('no generator xlsx member found')
gen_name=sorted(gen_names)[0]
xraw=z.read(gen_name); x=zipfile.ZipFile(io.BytesIO(xraw))
ns={'a':'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
shared=[]
if 'xl/sharedStrings.xml' in x.namelist():
    root=ET.fromstring(x.read('xl/sharedStrings.xml'))
    for si in root.findall('a:si',ns):
        shared.append(''.join(t.text or '' for t in si.findall('.//a:t',ns)))
def colnum(cell):
    m=re.match(r'([A-Z]+)',cell or 'A')
    n=0
    for ch in m.group(1): n=n*26+ord(ch)-64
    return n
def cellval(c):
    if c.attrib.get('t')=='inlineStr':
        return ''.join(t.text or '' for t in c.findall('.//a:t',ns)).strip()
    v=c.find('a:v',ns)
    if v is None: return None
    txt=v.text or ''
    if c.attrib.get('t')=='s' and txt != '':
        try: return shared[int(txt)].strip()
        except Exception: return txt.strip()
    return txt.strip()
def clean(v):
    if v is None: return None
    v=str(v).strip()
    if v in ('',' ','.','X','x'): return None
    return v
sheet='xl/worksheets/sheet1.xml'
root=ET.fromstring(x.read(sheet))
rows=[]
for row in root.findall('.//a:row',ns):
    vals={colnum(c.attrib.get('r','A1')):cellval(c) for c in row.findall('a:c',ns)}
    rows.append((int(row.attrib.get('r','0')), vals))
header=None; header_row=None
for rn, vals in rows:
    arr=[vals.get(i) for i in range(1, max(vals.keys(), default=0)+1)]
    if arr and arr[0]=='Utility ID' and 'Generator ID' in arr:
        header=[clean(x) or '' for x in arr]; header_row=rn; break
if not header: raise SystemExit('header row not found')
idx={h:i+1 for i,h in enumerate(header) if h}
def get(vals,name): return clean(vals.get(idx.get(name,-999)))
def sources(vals):
    out=[]
    for i in range(1,7):
        v=get(vals,f'Energy Source {i}')
        if v and v not in out: out.append(v)
    return out
records=[]; total=0; status_counts={}; tech_counts={}; state_counts={}; source_counts={}; sector_counts={}
early_note=None
if rows and rows[0][1]:
    early_note=clean(rows[0][1].get(1))
for rn, vals in rows:
    if header_row is None or rn <= header_row: continue
    if not get(vals,'Generator ID') or not get(vals,'Plant Code'): continue
    total += 1
    rec={
      'utility_id':get(vals,'Utility ID'),
      'utility_name':get(vals,'Utility Name'),
      'plant_code':get(vals,'Plant Code'),
      'plant_name':get(vals,'Plant Name'),
      'state':get(vals,'State'),
      'county':get(vals,'County'),
      'generator_id':get(vals,'Generator ID'),
      'technology':get(vals,'Technology'),
      'prime_mover':get(vals,'Prime Mover'),
      'unit_code':get(vals,'Unit Code'),
      'ownership':get(vals,'Ownership'),
      'status':get(vals,'Status'),
      'operating_month':get(vals,'Operating Month'),
      'operating_year':get(vals,'Operating Year'),
      'sector_name':get(vals,'Sector Name'),
      'sector_code':get(vals,'Sector'),
      'energy_sources':sources(vals),
      'precise_location_ingested':False,
      'capacity_values_ingested':False,
      'rto_iso_node_fields_ingested':False,
      'grid_synchronization_or_voltage_fields_ingested':False,
      'planned_modification_fields_ingested':False,
      'operational_guidance_ingested':False,
    }
    if len(records) < limit: records.append(rec)
    for key,d in [('status',status_counts),('technology',tech_counts),('state',state_counts),('sector_name',sector_counts)]:
        v=rec.get(key) or 'unknown'; d[v]=d.get(v,0)+1
    for s in rec['energy_sources'] or ['unknown']:
        source_counts[s]=source_counts.get(s,0)+1
def pairs(d,k): return [{k:x,'count':d[x]} for x in sorted(d)]
out_obj={
 'schema':'energy.eia860_generator_inventory.v1',
 'source':{
   'name':'EIA Form EIA-860 generator inventory',
   'license':'EIA public domain / acknowledgment requested',
   'source_urls':['https://www.eia.gov/electricity/data/eia860/','https://www.eia.gov/about/copyrights_reuse.php'],
   'receipt':receipt,
   'generated_at':receipt.get('retrieved_at'),
   'generator':'scripts/gen-energy-eia-860-generator-inventory.sh',
   'scope':'bounded generator identifier/classification metadata only; precise location, capacity/operational values, RTO/grid security fields, dispatch/maintenance guidance, forecast/trading advice, and graph/mirror wiring excluded'
 },
 'source_files':{
   'eia860_zip_sha256':hashlib.sha256(raw).hexdigest(),
   'generator_workbook_member':gen_name,
   'generator_workbook_sha256':hashlib.sha256(xraw).hexdigest(),
   'early_release_note_sha256':hashlib.sha256(early_note.encode('utf-8')).hexdigest() if early_note else None,
   'early_release_note_ingested':False,
 },
 'summary':{
   'generator_rows_total_available':total,
   'generator_rows_count':len(records),
   'limit':limit,
   'selected_year':receipt.get('selected_year'),
   'early_release':receipt.get('early_release'),
   'status_counts_total_available':pairs(status_counts,'status'),
   'technology_counts_total_available':pairs(tech_counts,'technology'),
   'state_counts_total_available':pairs(state_counts,'state'),
   'sector_counts_total_available':pairs(sector_counts,'sector_name'),
   'energy_source_counts_total_available':pairs(source_counts,'energy_source'),
   'precise_location_ingested':False,
   'capacity_values_ingested':False,
   'rto_iso_node_fields_ingested':False,
   'grid_synchronization_or_voltage_fields_ingested':False,
   'planned_modification_fields_ingested':False,
   'operational_dispatch_or_maintenance_guidance_ingested':False,
   'forecast_or_trading_advice_ingested':False,
   'mirror_graph_wiring':False,
 },
 'generators':records,
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
content='# stdlib/lib/corpus/eia-860-generator-inventory.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-energy-eia-860-generator-inventory.sh && scripts/gen-energy-eia-860-generator-inventory.sh\n'
content+='# 범위: EIA-860 generator identifier/classification metadata only. location/capacity/RTO/grid/security/operation guidance 제외.\n'
content+=pnix(out_obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True)
out.write_text(content,encoding='utf-8')
print(f'generated {out}: generators={len(records)}/{total} bytes={len(content.encode())}')
PY
