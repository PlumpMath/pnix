#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${FDC_API_CATALOG_OUT:-$ROOT/ingest/food/fdc-api-catalog}"
mkdir -p "$OUT"
receipt="$ROOT/corpus/food/LICENSES/fdc-api-catalog.legal-provenance-receipt.json"
bash "$ROOT/scripts/require-legal-provenance-gate.sh" "fdc-api-catalog" "$receipt"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
html="$tmp/api-guide.html"
code="$(curl -L --max-time 30 -s -o "$html" -w '%{http_code}' "${FDC_API_GUIDE_URL:-https://fdc.nal.usda.gov/api-guide.html}")"
python3 - "$html" "$code" "$OUT/pages.json" <<'PY'
import hashlib, html, json, re, sys
from pathlib import Path
from urllib.parse import urljoin, urlparse
path=Path(sys.argv[1]); code=int(sys.argv[2]); url='https://fdc.nal.usda.gov/api-guide.html'
body=path.read_bytes(); text=body.decode('utf-8','replace')
m=re.search(r'<title[^>]*>(.*?)</title>', text, re.I|re.S)
title=re.sub(r'\s+',' ',html.unescape(re.sub(r'<.*?>',' ',m.group(1)))).strip() if m else ''
refs=[]; seen=set()
for token in re.findall(r'(/fdc/v1/[A-Za-z0-9_/?&=\-{}]+)', text):
    t=token.split('&quot;')[0].split('"')[0].split('<')[0]
    if t not in seen:
        seen.add(t); refs.append({'path':t[:240]})
for href in re.findall(r'href=["\']([^"\']+)', text, re.I):
    u=urljoin(url, html.unescape(href)); pr=urlparse(u)
    if pr.netloc.endswith(('usda.gov','nal.usda.gov')):
        t=pr.path.strip('/')
        if t and t not in seen:
            seen.add(t); refs.append({'path':('/'+t)[:240]})
    if len(refs)>=80: break
out=Path(sys.argv[3])
out.write_text(json.dumps({'schema':'food.fdc_api_catalog.raw.v1','retrieved_at':'2026-06-20','page':{'url':url,'http_status':code,'title':title,'sha256':hashlib.sha256(body).hexdigest(),'api_path_refs':refs[:80]}}, indent=2, sort_keys=True)+'\n')
print(f'fdc-api-catalog updated: {out} refs={len(refs[:80])}')
PY
( cd "$OUT" && shasum -a 256 pages.json > SHA256SUMS )
