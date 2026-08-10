#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${NRC_PUBLIC_CATALOG_OUT:-$ROOT/ingest/science/nrc-public-catalog}"
mkdir -p "$OUT"
receipt="$ROOT/corpus/science/LICENSES/nrc-public-catalog.legal-provenance-receipt.json"
bash "$ROOT/scripts/require-legal-provenance-gate.sh" "nrc-public-catalog" "$receipt"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
cat > "$tmp/pages.tsv" <<EOF
event_notification_index	https://www.nrc.gov/reading-rm/doc-collections/event-status/event/
reactor_oversight_process	https://www.nrc.gov/reactors/operating/oversight.html
EOF
: > "$tmp/fetched.tsv"
while IFS=$'\t' read -r sid url; do
  html="$tmp/${sid}.html"
  code="$(curl -L --max-time 20 -s -o "$html" -w '%{http_code}' "$url")"
  printf '%s\t%s\t%s\t%s\n' "$sid" "$url" "$code" "$html" >> "$tmp/fetched.tsv"
done < "$tmp/pages.tsv"
python3 - "$tmp/fetched.tsv" "$OUT/pages.json" <<'PY'
import hashlib, html, json, re, sys
from pathlib import Path
from urllib.parse import urljoin, urlparse
rows=[]
for line in Path(sys.argv[1]).read_text().splitlines():
    sid,url,code,path=line.split('\t')
    body=Path(path).read_bytes()
    text=body.decode('utf-8','replace')
    title=''
    m=re.search(r'<title[^>]*>(.*?)</title>', text, re.I|re.S)
    if m: title=re.sub(r'\s+',' ',html.unescape(re.sub(r'<.*?>',' ',m.group(1)))).strip()
    links=[]; seen=set()
    for href in re.findall(r'href=["\']([^"\'#?]+)', text, re.I):
        u=urljoin(url, html.unescape(href))
        pr=urlparse(u)
        if pr.netloc and pr.netloc.endswith('nrc.gov'):
            token=pr.path.strip('/')
            if token and token not in seen:
                seen.add(token); links.append({'path':token[:220]})
        if len(links)>=80: break
    rows.append({'source_id':sid,'url':url,'http_status':int(code),'title':title,'sha256':hashlib.sha256(body).hexdigest(),'internal_links':links})
out=Path(sys.argv[2])
out.write_text(json.dumps({'schema':'radiation.nrc_public_catalog.raw.v1','retrieved_at':'2026-06-20','pages':rows}, indent=2, sort_keys=True)+'\n')
print(f'nrc-public-catalog updated: {out} pages={len(rows)}')
PY
( cd "$OUT" && shasum -a 256 pages.json > SHA256SUMS )
