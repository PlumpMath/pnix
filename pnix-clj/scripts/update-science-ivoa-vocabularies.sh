#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${IVOA_VOCAB_SRC:-$ROOT/ingest/science/ivoa-vocabularies}"
LIMIT="${IVOA_VOCAB_LIMIT:-0}"
INDEX_URL="${IVOA_VOCAB_INDEX_URL:-https://www.ivoa.net/rdf/}"
mkdir -p "$DEST/raw"
python3 - "$DEST" "$INDEX_URL" "$LIMIT" <<'PY'
import datetime as dt, hashlib, html, json, os, re, sys, urllib.request
from pathlib import Path

dest=Path(sys.argv[1]); index_url=sys.argv[2]; limit=int(sys.argv[3])
raw=dest/'raw'; raw.mkdir(parents=True, exist_ok=True)

def fetch(url, accept=None):
    req=urllib.request.Request(url, headers={'User-Agent':'pnix-ingest/ivoa-vocabularies'} | ({'Accept':accept} if accept else {}))
    with urllib.request.urlopen(req, timeout=30) as r:
        return r.read(), dict(r.headers), r.geturl()

index_bytes, index_headers, final_index = fetch(index_url)
(index_path:=raw/'index.html').write_bytes(index_bytes)
text=index_bytes.decode('utf-8','replace')
rows=[]
for m in re.finditer(r'<tr class="status-([^"]+)">(.*?)</tr>', text, re.I|re.S):
    status=m.group(1)
    row=m.group(2)
    href_m=re.search(r'<a href="(http://www\.ivoa\.net/rdf/[^"]+)">', row)
    if not href_m:
        continue
    title_m=re.search(r'<td>(.*?)<br\s*/?>', row, re.I|re.S)
    date_m=re.search(r'<td class="date-cell">([^<]+)</td>', row)
    title=html.unescape(re.sub('<[^>]+>',' ', title_m.group(1))).strip() if title_m else ''
    url=href_m.group(1).replace('http://www.ivoa.net','https://www.ivoa.net')
    slug=url.rsplit('/rdf/',1)[1].replace('/','__')
    rows.append({'id':slug,'title':title,'status':status,'date':date_m.group(1).strip() if date_m else '', 'url':url})
if limit>0:
    rows=rows[:limit]
files=[]
for row in rows:
    data, headers, final_url = fetch(row['url'], 'text/turtle')
    p=raw/(row['id']+'.ttl')
    p.write_bytes(data)
    files.append({
        'id':row['id'], 'title':row['title'], 'status':row['status'], 'date':row['date'],
        'url':row['url'], 'final_url':final_url, 'path':str(p.relative_to(dest)),
        'sha256':hashlib.sha256(data).hexdigest(), 'size_bytes':len(data),
        'content_type':headers.get('Content-Type','')
    })
receipt={
    'schema':'pnix.ingest.source_receipt.v1',
    'source':'IVOA Vocabularies',
    'license':'CC0 1.0 unless specified otherwise on the vocabulary page',
    'index_url':index_url,
    'final_index_url':final_index,
    'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),
    'index_sha256':hashlib.sha256(index_bytes).hexdigest(),
    'index_size_bytes':len(index_bytes),
    'scope':'official IVOA vocabulary TTL files; generated rows keep vocabulary/term identifiers, labels, and structural relations only; prose definitions/descriptions/comments/examples/observation payloads/graph wiring excluded',
    'vocabulary_count':len(files),
    'files':files,
}
(dest/'source-receipt.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True)+'\n')
print(f"updated IVOA vocabularies: vocabularies={len(files)} dest={dest}")
PY
