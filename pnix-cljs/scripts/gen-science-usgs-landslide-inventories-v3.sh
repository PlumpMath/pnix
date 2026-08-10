#!/usr/bin/env bash
# USGS Landslide Inventories v3 metadata -> pnix attrset source.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC_DIR="${USGS_LANDSLIDE_V3_SRC:-$ROOT/ingest/science/usgs-landslide-inventories-v3}"
OUT="${OUT:-$ROOT/stdlib/lib/corpus/usgs-landslide-inventories-v3.generated.px}"
ITEM="$SRC_DIR/sciencebase-item.json"
REFS="$SRC_DIR/us_ls_v3_references.csv"
RECEIPT="$SRC_DIR/source-receipt.json"
if [[ ! -f "$ITEM" || ! -f "$REFS" ]]; then
  echo "missing USGS Landslide v3 source files; run scripts/update-science-usgs-landslide-inventories-v3.sh first" >&2
  exit 2
fi
python3 - "$ITEM" "$REFS" "$RECEIPT" "$OUT" <<'PY'
import csv, hashlib, io, json, pathlib, re, sys, urllib.parse
item_path, refs_path, receipt_path, out_path = map(pathlib.Path, sys.argv[1:])
obj=json.loads(item_path.read_text(encoding='utf-8'))
try: receipt=json.loads(receipt_path.read_text(encoding='utf-8'))
except Exception: receipt={}
refs_text=refs_path.read_text(encoding='utf-8-sig',errors='replace')
refs_rows=list(csv.DictReader(io.StringIO(refs_text)))
url_re=re.compile(r'https?://[^\s,;\])]+')
def compact(v):
    return None if v in (None,'') else v
def safe_file(f):
    return {
      'name': compact(f.get('name')),
      'content_type': compact(f.get('contentType')),
      'size_bytes': f.get('size'),
      'date_uploaded': compact(f.get('dateUploaded')),
      'download_uri': compact(f.get('downloadUri') or f.get('url')),
      'body_ingested': False,
    }
def ref_row(r):
    ref=r.get('Reference') or ''
    urls=url_re.findall(ref)
    years=sorted(set(re.findall(r'\b(?:19|20)\d{2}\b', ref)))
    return {
      'inventory': compact(r.get('Inventory')),
      'reference_sha256': hashlib.sha256(ref.encode('utf-8')).hexdigest(),
      'urls': urls,
      'years': years,
      'reference_text_ingested': False,
    }
ids=[]
for ident in obj.get('identifiers') or []:
    ids.append({k:ident.get(k) for k in sorted(ident) if ident.get(k) not in (None,'')})
dates=[]
for d in obj.get('dates') or []:
    dates.append({k:d.get(k) for k in sorted(d) if d.get(k) not in (None,'')})
files=[safe_file(f) for f in (obj.get('files') or [])]
large_excluded=[f['name'] for f in files if (f.get('size_bytes') or 0) > 1000000]
refs=[ref_row(r) for r in refs_rows]
obj_out={
 'schema':'science.usgs.landslide_inventories_v3.release_metadata.v1',
 'source':{
   'name':'USGS Landslide Inventories across the United States (ver. 3.0, February 2025)',
   'license':'US Government public domain',
   'source_urls':['https://www.usgs.gov/data/landslide-inventories-across-united-states-ver-30-february-2025','https://www.sciencebase.gov/catalog/item/671eef1fd34ed0f827ea9f12','https://doi.org/10.5066/P14AJF8I'],
   'receipt':receipt,
   'generated_at':receipt.get('retrieved_at'),
   'generator':'scripts/gen-science-usgs-landslide-inventories-v3.sh',
   'scope':'ScienceBase item/file metadata + references CSV URL/hash structure only; landslide geometry/event rows/ancillary/analyses/safety judgments excluded'
 },
 'release':{
   'sciencebase_item_id':obj.get('id'),
   'title':obj.get('title'),
   'doi':'10.5066/P14AJF8I',
   'link':obj.get('link',{}).get('url') if isinstance(obj.get('link'),dict) else obj.get('link'),
   'identifiers':ids,
   'dates':dates,
   'citation_sha256':hashlib.sha256((obj.get('citation') or '').encode('utf-8')).hexdigest() if obj.get('citation') else None,
   'summary_sha256':hashlib.sha256((obj.get('summary') or '').encode('utf-8')).hexdigest() if obj.get('summary') else None,
   'body_sha256':hashlib.sha256((obj.get('body') or '').encode('utf-8')).hexdigest() if obj.get('body') else None,
   'citation_text_ingested':False,
   'summary_text_ingested':False,
   'body_text_ingested':False,
 },
 'summary':{
   'file_count':len(files),
   'reference_count':len(refs),
   'large_payload_files_excluded':large_excluded,
   'landslide_geometry_ingested':False,
   'landslide_event_rows_ingested':False,
   'ancillary_fields_ingested':False,
   'analyses_ingested':False,
   'safety_guidance_ingested':False,
   'mirror_graph_wiring':False,
 },
 'files':files,
 'references':refs,
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
content='# stdlib/lib/corpus/usgs-landslide-inventories-v3.generated.px — GENERATED, do not commit.\n'
content+='# 생성: scripts/update-science-usgs-landslide-inventories-v3.sh && scripts/gen-science-usgs-landslide-inventories-v3.sh\n'
content+='# 범위: ScienceBase release/file/reference metadata only. landslide geometry/event rows/analyses/safety judgments 제외.\n'
content+=pnix(obj_out)+'\n'
out_path.parent.mkdir(parents=True,exist_ok=True)
out_path.write_text(content,encoding='utf-8')
print(f"generated {out_path}: files={len(files)} references={len(refs)} bytes={len(content.encode())}")
PY
