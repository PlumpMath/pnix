#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$ROOT/ingest/science/noaa-swpc-data-access/raw"
RECEIPT="$ROOT/ingest/science/noaa-swpc-data-access/source-receipt.json"
OUT="$ROOT/stdlib/lib/corpus/noaa-swpc-data-access.generated.px"
if [[ ! -d "$SRC" ]]; then echo "missing $SRC; run update first" >&2; exit 1; fi
python3 - "$SRC" "$OUT" "$RECEIPT" <<'PY'
import html, json, re, sys, urllib.parse
from pathlib import Path
src=Path(sys.argv[1]); out=Path(sys.argv[2]); receipt=json.loads(Path(sys.argv[3]).read_text())
def esc(s): return json.dumps(str(s), ensure_ascii=False)
def to_pnix(v):
    if isinstance(v,bool): return 'true' if v else 'false'
    if isinstance(v,int): return str(v)
    if isinstance(v,list): return '[ ' + ' '.join(to_pnix(x) for x in v) + ' ]'
    if isinstance(v,dict): return '{ ' + ' '.join(f'{k} = {to_pnix(val)};' for k,val in v.items() if val is not None) + ' }'
    return esc(v)
files=[]; entries=[]; services=[]
source_by_file={Path(s['file']).name:s['url'] for s in receipt.get('sources',[])}
for path in sorted(src.glob('*.html')):
    url=source_by_file.get(path.name,'')
    text=path.read_text(errors='replace')
    files.append({'file':path.name,'url':url,'bytes':path.stat().st_size,'lines':len(text.splitlines())})
    if 'services.swpc.noaa.gov' not in url: continue
    parsed_base=urllib.parse.urlparse(url)
    base_path=parsed_base.path
    service_root=base_path.strip('/').split('/')[0] if base_path.strip('/') else ''
    if service_root and service_root not in services: services.append(service_root)
    # Apache/nginx index row: <a href="name">name</a> date size
    for m in re.finditer(r'<a\s+href="([^"]+)">([^<]+)</a>\s*([^<\n]*)', text, flags=re.I):
        href=html.unescape(m.group(1)); name=html.unescape(m.group(2)).strip(); tail=' '.join(html.unescape(m.group(3)).split())
        if href in ('../','/') or name == 'Parent Directory': continue
        full=urllib.parse.urljoin(url if url.endswith('/') else url+'/', href)
        p=urllib.parse.urlparse(full)
        ext=''
        if '.' in name and not name.endswith('/'):
            ext=name.rsplit('.',1)[-1].lower()
        kind='directory' if href.endswith('/') else ('json' if ext=='json' else 'text' if ext in ('txt','csv','xml') else 'image' if ext in ('png','jpg','jpeg','gif') else 'file')
        entries.append({'source_file':path.name,'service':service_root,'directory':base_path,'name':name.rstrip('/'),'href':href,'url':full,'kind':kind,'extension':ext,'index_tail':tail})
data={
 'schema':'space_weather.noaa_swpc.data_access.v1',
 'source':'NOAA SWPC public data service directory metadata',
 'license':'USGOV-PUBLIC-METADATA',
 'scope':'directory index metadata only; no product payloads fetched',
 'source_files':files,
 'service_roots':[{'name':s} for s in sorted(set(services))],
 'entries':entries,
 'exclusions':['json/text/image product payloads','forecasts','alerts','observations','operational guidance','credentials','execution','mirror/graph wiring']
}
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(to_pnix(data)+'\n')
print(f"generated {out}: files={len(files)} service_roots={len(set(services))} entries={len(entries)} bytes={out.stat().st_size}")
PY
