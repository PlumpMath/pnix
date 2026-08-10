#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${CDC_WONDER_PLACES_OUT:-$ROOT/ingest/public_health/cdc-wonder-places-catalog}"
mkdir -p "$OUT"
receipt="$ROOT/corpus/public_health/LICENSES/cdc-wonder-places-catalog.legal-provenance-receipt.json"
bash "$ROOT/scripts/require-legal-provenance-gate.sh" "cdc-wonder-places-catalog" "$receipt"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
cat > "$tmp/pages.tsv" <<EOF
cdc_wonder_home|https://wonder.cdc.gov/
cdc_places_home|https://www.cdc.gov/places/
EOF
: > "$tmp/fetched.tsv"
while IFS='|' read -r sid url; do
  html="$tmp/${sid}.html"
  code="$(curl -L --max-time 30 -s -o "$html" -w '%{http_code}' "$url")"
  printf '%s\t%s\t%s\t%s\n' "$sid" "$url" "$code" "$html" >> "$tmp/fetched.tsv"
done < "$tmp/pages.tsv"
curl -L --fail --retry 3 --retry-delay 2 --max-time 30 -o "$OUT/places-view.json" "${PLACES_VIEW_URL:-https://data.cdc.gov/api/views/cwsq-ngmh}"
python3 - "$tmp/fetched.tsv" "$OUT/pages.json" <<'PY'
import hashlib, html, json, re, sys
from pathlib import Path
from urllib.parse import urljoin, urlparse
rows=[]
for line in Path(sys.argv[1]).read_text().splitlines():
    sid,url,code,path=line.split('\t')
    body=Path(path).read_bytes(); text=body.decode('utf-8','replace')
    m=re.search(r'<title[^>]*>(.*?)</title>', text, re.I|re.S)
    title=re.sub(r'\s+',' ',html.unescape(re.sub(r'<.*?>',' ',m.group(1)))).strip() if m else ''
    links=[]; seen=set()
    for href in re.findall(r'href=["\']([^"\']+)', text, re.I):
        u=urljoin(url, html.unescape(href)); pr=urlparse(u)
        if pr.netloc.endswith(('cdc.gov','data.cdc.gov')):
            token=pr.path.strip('/')
            if token and token not in seen:
                seen.add(token); links.append({'host':pr.netloc,'path':token[:260]})
        if len(links)>=80: break
    rows.append({'source_id':sid,'url':url,'http_status':int(code),'title':title,'sha256':hashlib.sha256(body).hexdigest(),'link_refs':links})
out=Path(sys.argv[2])
out.write_text(json.dumps({'schema':'public_health.cdc_pages.raw.v1','retrieved_at':'2026-06-20','pages':rows}, indent=2, sort_keys=True)+'\n')
print(f'cdc-wonder-places pages updated: {out} pages={len(rows)}')
PY
( cd "$OUT" && shasum -a 256 pages.json places-view.json > SHA256SUMS )
