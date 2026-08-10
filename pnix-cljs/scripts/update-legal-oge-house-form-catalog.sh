#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${OGE_HOUSE_FORM_OUT:-$ROOT/ingest/legal/oge-house-form-catalog}"
mkdir -p "$OUT"
receipt="$ROOT/corpus/legal/LICENSES/oge-house-form-catalog.legal-provenance-receipt.json"
bash "$ROOT/scripts/require-legal-provenance-gate.sh" "oge-house-form-catalog" "$receipt"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
cat > "$tmp/pages.tsv" <<EOF
oge_public_guide|https://www.oge.gov/web/OGE.nsf/Resources/Public+Financial+Disclosure+Guide
house_ethics_forms|https://ethics.house.gov/financial-disclosure/forms
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
    refs=[]; seen=set()
    for href in re.findall(r'href=["\']([^"\']+)', text, re.I):
        u=urljoin(url, html.unescape(href))
        pr=urlparse(u)
        low=pr.path.lower()
        if re.search(r'\.(pdf|docx?|xlsx?)$', low) or 'form' in low:
            token=(pr.netloc + pr.path).strip('/')
            if token and token not in seen:
                seen.add(token); refs.append({'host':pr.netloc,'path':pr.path[:260]})
        if len(refs)>=80: break
    rows.append({'source_id':sid,'url':url,'http_status':int(code),'title':title,'sha256':hashlib.sha256(body).hexdigest(),'form_link_refs':refs})
out=Path(sys.argv[2])
out.write_text(json.dumps({'schema':'legal.oge_house_form_catalog.raw.v1','retrieved_at':'2026-06-20','pages':rows}, indent=2, sort_keys=True)+'\n')
print(f'oge-house-form-catalog updated: {out} pages={len(rows)}')
PY
( cd "$OUT" && shasum -a 256 pages.json > SHA256SUMS )
