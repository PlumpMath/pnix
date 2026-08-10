#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${NIOSH_OSHA_REF_OUT:-$ROOT/ingest/tox/niosh-osha-ref-catalog}"
mkdir -p "$OUT"
receipt="$ROOT/corpus/tox/LICENSES/niosh-osha-ref-catalog.legal-provenance-receipt.json"
bash "$ROOT/scripts/require-legal-provenance-gate.sh" "niosh-osha-ref-catalog" "$receipt"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
cat > "$tmp/pages.tsv" <<EOF
niosh_npg_index|https://www.cdc.gov/niosh/npg/
osha_annotated_pels|https://www.osha.gov/annotated-pels
osha_chemical_database|https://www.osha.gov/chemicaldata
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
    links=[]; seen=set()
    for href in re.findall(r'href=["\']([^"\']+)', text, re.I):
        u=urljoin(url, html.unescape(href)); pr=urlparse(u)
        if pr.netloc.endswith(('cdc.gov','osha.gov')):
            path_token=pr.path.strip('/')
            if path_token and path_token not in seen:
                seen.add(path_token); links.append({'host':pr.netloc,'path':path_token[:260]})
        if len(links)>=100: break
    rows.append({'source_id':sid,'url':url,'http_status':int(code),'title':title,'sha256':hashlib.sha256(body).hexdigest(),'link_refs':links})
out=Path(sys.argv[2])
out.write_text(json.dumps({'schema':'tox.niosh_osha_ref_catalog.raw.v1','retrieved_at':'2026-06-20','pages':rows}, indent=2, sort_keys=True)+'\n')
print(f'niosh-osha-ref-catalog updated: {out} pages={len(rows)}')
PY
( cd "$OUT" && shasum -a 256 pages.json > SHA256SUMS )
