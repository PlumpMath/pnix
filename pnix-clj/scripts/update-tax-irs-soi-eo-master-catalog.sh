#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${IRS_SOI_EO_MASTER_OUT:-$ROOT/ingest/tax/irs-soi-eo-master-catalog}"
mkdir -p "$OUT"
receipt="$ROOT/corpus/tax/LICENSES/irs-soi-eo-master-catalog.legal-provenance-receipt.json"
bash "$ROOT/scripts/require-legal-provenance-gate.sh" "irs-soi-eo-master-catalog" "$receipt"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
cat > "$tmp/pages.tsv" <<EOF
soi_zip_code_data|https://www.irs.gov/statistics/soi-tax-stats-individual-income-tax-statistics-zip-code-data-soi
eo_bmf_extract|https://www.irs.gov/charities-non-profits/exempt-organizations-business-master-file-extract-eo-bmf
EOF
: > "$tmp/fetched.tsv"
while IFS='|' read -r sid url; do
  html="$tmp/${sid}.html"
  code="$(curl -L --max-time 30 -s -o "$html" -w '%{http_code}' "$url")"
  printf '%s\t%s\t%s\t%s\n' "$sid" "$url" "$code" "$html" >> "$tmp/fetched.tsv"
done < "$tmp/pages.tsv"
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
    files=[]; seen=set()
    for href in re.findall(r'href=["\']([^"\']+)', text, re.I):
        u=urljoin(url, html.unescape(href))
        pr=urlparse(u)
        low=pr.path.lower()
        if pr.netloc.endswith('irs.gov') and re.search(r'\.(csv|xlsx?|zip|txt)$', low):
            token=pr.path.strip('/')
            if token and token not in seen:
                seen.add(token); files.append({'path':token[:260],'ext':low.rsplit('.',1)[-1]})
        if len(files)>=120: break
    rows.append({'source_id':sid,'url':url,'http_status':int(code),'title':title,'sha256':hashlib.sha256(body).hexdigest(),'download_file_refs':files})
out=Path(sys.argv[2])
out.write_text(json.dumps({'schema':'tax.irs_soi_eo_master_catalog.raw.v1','retrieved_at':'2026-06-20','pages':rows}, indent=2, sort_keys=True)+'\n')
print(f'irs-soi-eo-master-catalog updated: {out} pages={len(rows)}')
PY
( cd "$OUT" && shasum -a 256 pages.json > SHA256SUMS )
