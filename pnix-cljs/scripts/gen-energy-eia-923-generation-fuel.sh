#!/usr/bin/env bash
# EIA-923 generation/fuel workbook -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${EIA923_SRC:-$ROOT/ingest/energy/eia-923-generation-fuel/eia923.zip}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/eia-923-generation-fuel.generated.px}"
RECEIPT="$ROOT/ingest/energy/eia-923-generation-fuel/source-receipt.json"
LIMIT="${EIA923_LIMIT:-250}"
if [[ ! -f "$SRC" ]]; then
  echo "missing EIA-923 zip: $SRC" >&2
  echo "run scripts/update-energy-eia-923-generation-fuel.sh first" >&2
  exit 2
fi
python3 - "$SRC" "$OUT" "$RECEIPT" "$LIMIT" <<'PY'
import decimal, hashlib, io, json, pathlib, re, sys, zipfile, xml.etree.ElementTree as ET
src, out, receipt_path = map(pathlib.Path, sys.argv[1:4]); limit=int(sys.argv[4])
raw=src.read_bytes()
try: receipt=json.loads(receipt_path.read_text(encoding='utf-8'))
except Exception: receipt={}
z=zipfile.ZipFile(io.BytesIO(raw))
xlsx_names=[n for n in z.namelist() if n.lower().endswith('.xlsx') and 'schedule' in n.lower()]
if not xlsx_names: xlsx_names=[n for n in z.namelist() if n.lower().endswith('.xlsx')]
if not xlsx_names: raise SystemExit('no xlsx member found')
xlsx_name=sorted(xlsx_names)[0]
xraw=z.read(xlsx_name); x=zipfile.ZipFile(io.BytesIO(xraw))
ns={'a':'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}
shared=[]
if 'xl/sharedStrings.xml' in x.namelist():
    root=ET.fromstring(x.read('xl/sharedStrings.xml'))
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
    v=str(v).strip()
    if v in ('','.',' '): return None
    return v
def num(v):
    v=clean(v)
    if v is None: return None
    try:
        d=decimal.Decimal(v)
        if d == d.to_integral_value(): return int(d)
        return float(d)
    except Exception: return None
sheet='xl/worksheets/sheet1.xml'
root=ET.fromstring(x.read(sheet))
rows=[]
for row in root.findall('.//a:row',ns):
    vals={colnum(c.attrib.get('r','A1')):cellval(c) for c in row.findall('a:c',ns)}
    rows.append((int(row.attrib.get('r','0')), vals))
header=None; header_row=None
for rn, vals in rows:
    arr=[vals.get(i) for i in range(1, max(vals.keys(), default=0)+1)]
    if arr and arr[0]=='Plant Id' and 'Net Generation\n(Megawatthours)' in arr:
        header=[clean(x) or '' for x in arr]; header_row=rn; break
if not header: raise SystemExit('Page 1 header row not found')
idx={h:i+1 for i,h in enumerate(header) if h}
def get(vals,name): return clean(vals.get(idx.get(name,-999)))
records=[]; total=0; state_counts={}; fuel_counts={}; prime_counts={}; sector_counts={}; nerc_counts={}
for rn, vals in rows:
    if header_row is None or rn <= header_row: continue
    if not get(vals,'Plant Id') or not get(vals,'Reported\nFuel Type Code'): continue
    total += 1
    rec={
      'plant_id':get(vals,'Plant Id'),
      'plant_name':get(vals,'Plant Name'),
      'operator_id':get(vals,'Operator Id'),
      'operator_name':get(vals,'Operator Name'),
      'plant_state':get(vals,'Plant State'),
      'census_region':get(vals,'Census Region'),
      'nerc_region':get(vals,'NERC Region'),
      'naics_code':get(vals,'NAICS Code'),
      'eia_sector_number':get(vals,'EIA Sector Number'),
      'sector_name':get(vals,'Sector Name'),
      'reported_prime_mover':get(vals,'Reported\nPrime Mover'),
      'reported_fuel_type_code':get(vals,'Reported\nFuel Type Code'),
      'mer_fuel_type_code':get(vals,'MER\nFuel Type Code'),
      'balancing_authority_code':get(vals,'Balancing\nAuthority Code'),
      'physical_unit_label':get(vals,'Physical\nUnit Label'),
      'year':get(vals,'YEAR'),
      'total_fuel_consumption_quantity':num(get(vals,'Total Fuel Consumption\nQuantity')),
      'electric_fuel_consumption_quantity':num(get(vals,'Electric Fuel Consumption\nQuantity')),
      'total_fuel_consumption_mmbtu':num(get(vals,'Total Fuel Consumption\nMMBtu')),
      'electric_fuel_consumption_mmbtu':num(get(vals,'Elec Fuel Consumption\nMMBtu')),
      'net_generation_mwh':num(get(vals,'Net Generation\n(Megawatthours)')),
      'monthly_vectors_ingested':False,
      'operational_dispatch_guidance_ingested':False,
      'facility_security_sensitive_payload_ingested':False,
    }
    if len(records) < limit: records.append(rec)
    for key,d in [('plant_state',state_counts),('reported_fuel_type_code',fuel_counts),('reported_prime_mover',prime_counts),('sector_name',sector_counts),('nerc_region',nerc_counts)]:
        v=rec.get(key) or 'unknown'; d[v]=d.get(v,0)+1
def pairs(d,k): return [{k:x,'count':d[x]} for x in sorted(d)]
out_obj={
 'schema':'energy.eia923_generation_fuel.v1',
 'source':{
   'name':'EIA Form EIA-923 generation and fuel data',
   'license':'EIA public domain / acknowledgment requested',
   'source_urls':['https://www.eia.gov/electricity/data/eia923/','https://www.eia.gov/about/copyrights_reuse.php'],
   'receipt':receipt,
   'generated_at':receipt.get('retrieved_at'),
   'generator':'scripts/gen-energy-eia-923-generation-fuel.sh',
   'scope':'bounded annual aggregate generation/fuel rows only; monthly vectors, dispatch/control guidance, security-sensitive detail, forecast/trading advice, and graph/mirror wiring excluded'
 },
 'source_files':{
   'eia923_zip_sha256':hashlib.sha256(raw).hexdigest(),
   'workbook_member':xlsx_name,
   'workbook_sha256':hashlib.sha256(xraw).hexdigest(),
 },
 'summary':{
   'generation_fuel_rows_total_available':total,
   'generation_fuel_rows_count':len(records),
   'limit':limit,
   'selected_year':receipt.get('selected_year'),
   'skipped_invalid_candidates':receipt.get('skipped_invalid_candidates'),
   'state_counts_total_available':pairs(state_counts,'plant_state'),
   'fuel_type_counts_total_available':pairs(fuel_counts,'reported_fuel_type_code'),
   'prime_mover_counts_total_available':pairs(prime_counts,'reported_prime_mover'),
   'sector_counts_total_available':pairs(sector_counts,'sector_name'),
   'nerc_region_counts_total_available':pairs(nerc_counts,'nerc_region'),
   'monthly_vectors_ingested':False,
   'dispatch_or_control_guidance_ingested':False,
   'facility_security_sensitive_payload_ingested':False,
   'forecast_or_trading_advice_ingested':False,
   'mirror_graph_wiring':False,
 },
 'generation_fuel_rows':records,
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
content='# stdlib/lib/corpus/eia-923-generation-fuel.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-energy-eia-923-generation-fuel.sh && scripts/gen-energy-eia-923-generation-fuel.sh\n'
content+='# 범위: EIA-923 annual aggregate generation/fuel rows only. monthly vectors/dispatch/security/forecast guidance 제외.\n'
content+=pnix(out_obj)+'\n'
out.parent.mkdir(parents=True,exist_ok=True)
out.write_text(content,encoding='utf-8')
print(f'generated {out}: rows={len(records)}/{total} bytes={len(content.encode())}')
PY
