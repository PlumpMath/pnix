#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/science/noaa-swpc-data-access"
mkdir -p "$OUT/raw"
rm -f "$OUT/raw"/*.html
base_urls=(
  "https://www.swpc.noaa.gov/content/data-access"
  "https://services.swpc.noaa.gov/json/"
  "https://services.swpc.noaa.gov/products/"
  "https://services.swpc.noaa.gov/text/"
)
fetch_one() {
  local url="$1"
  local name
  name="$(echo "$url" | sed 's#[^A-Za-z0-9]#_#g').html"
  curl -L --fail --retry 3 --connect-timeout 20 -o "$OUT/raw/$name" "$url"
  printf '%s\t%s\n' "$url" "$name"
}
manifest="$OUT/fetched.tsv"
: > "$manifest"
for u in "${base_urls[@]}"; do fetch_one "$u" >> "$manifest" || true; done
# Fetch 1-depth subdirectory indexes only. Never fetch non-directory product payloads.
python3 - "$OUT/raw" "$manifest" <<'PY' | while IFS=$'\t' read -r url name; do
import html, re, sys, urllib.parse
from pathlib import Path
raw=Path(sys.argv[1]); manifest=Path(sys.argv[2])
seen=set(line.split('\t',1)[0] for line in manifest.read_text().splitlines() if line.strip())
for line in manifest.read_text().splitlines():
    if not line.strip(): continue
    url,name=line.split('\t',1)
    if 'services.swpc.noaa.gov' not in url: continue
    text=(raw/name).read_text(errors='replace')
    base=url if url.endswith('/') else url+'/'
    for href in re.findall(r'<a\s+href="([^"]+)"', text, flags=re.I):
        href=html.unescape(href)
        if href in ('../','/') or not href.endswith('/'): continue
        sub=urllib.parse.urljoin(base, href)
        # only one level below known roots
        path=urllib.parse.urlparse(sub).path.strip('/').split('/')
        if len(path) > 2: continue
        if sub not in seen:
            seen.add(sub)
            safe=re.sub(r'[^A-Za-z0-9]','_',sub)+'.html'
            print(sub+'\t'+safe)
PY
  curl -L --fail --retry 3 --connect-timeout 20 -o "$OUT/raw/$name" "$url" || true
  printf '%s\t%s\n' "$url" "$name" >> "$manifest"
done
python3 - "$OUT/raw" "$manifest" > "$OUT/source-receipt.json" <<'PY'
import json, sys, hashlib, datetime
from pathlib import Path
raw=Path(sys.argv[1]); manifest=Path(sys.argv[2])
sources=[]
for line in manifest.read_text().splitlines():
    if not line.strip(): continue
    url,name=line.split('\t',1); p=raw/name
    if not p.exists(): continue
    sources.append({'url':url,'file':'raw/'+name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
print(json.dumps({
 'schema':'pnix.ingest.source_receipt.v1',
 'source_id':'noaa-swpc-data-access',
 'source_name':'NOAA SWPC Data Access metadata',
 'retrieved_at_utc':datetime.datetime.utcnow().replace(microsecond=0).isoformat()+'Z',
 'license':'USGOV-PUBLIC-METADATA',
 'sources':sources,
 'scope':'official SWPC data-access page plus directory index metadata only',
 'excluded':['json/text/image product payloads','forecasts','alerts','observations','operational guidance','credentials','execution','mirror/graph wiring']
}, indent=2))
PY
printf 'updated %s: html_files=%s\n' "$OUT" "$(find "$OUT/raw" -type f -name '*.html' | wc -l | tr -d ' ')" >&2
