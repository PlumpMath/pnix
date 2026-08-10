#!/usr/bin/env bash
# OpenCitations Figshare dump manifest snapshot.
# Fetches article metadata only. Does not download RDF/CSV dump payloads and does not query APIs/SPARQL.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${OPENCITATIONS_DEST:-$ROOT/ingest/scholarly/opencitations-dump-manifest}"
ARTICLE_IDS="${OPENCITATIONS_FIGSHARE_ARTICLE_IDS:-31353691,28677293,21747461}"
rm -rf "$DEST"
mkdir -p "$DEST/raw"
python3 - "$DEST" "$ARTICLE_IDS" <<'PY'
import datetime as dt, hashlib, json, pathlib, sys, urllib.request
out=pathlib.Path(sys.argv[1]); ids=[x.strip() for x in sys.argv[2].split(',') if x.strip()]
articles=[]; files=[]; source_file_rows=[]
for article_id in ids:
    api_url=f'https://api.figshare.com/v2/articles/{article_id}'
    with urllib.request.urlopen(api_url, timeout=60) as r:
        raw=r.read()
    article=json.loads(raw.decode('utf-8'))
    rel=pathlib.Path('raw')/(article_id + '.json')
    (out/rel).write_bytes(raw)
    source_file_rows.append({'source_path':str(rel),'relative_path':str(rel),'url':api_url,'sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw),'role':'figshare_article_metadata_json','article_id':article_id})
    article_files=[]
    for f in article.get('files') or []:
        row={'article_id':article_id,'id':f.get('id'),'name':f.get('name'),'size':f.get('size'),'download_url':f.get('download_url'),'computed_md5':f.get('computed_md5'),'supplied_md5':f.get('supplied_md5')}
        files.append(row); article_files.append(row)
    articles.append({'article_id':article_id,'title':article.get('title'),'doi':article.get('doi'),'version':article.get('version'),'published_date':article.get('published_date'),'modified_date':article.get('modified_date'),'defined_type_name':article.get('defined_type_name'),'license':article.get('license'),'figshare_url':article.get('figshare_url'),'url_public_api':article.get('url_public_api'),'file_count':len(article_files),'total_file_size_bytes':sum((f.get('size') or 0) for f in article_files)})
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'OpenCitations dump Figshare article/file manifests','article_ids':ids,'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'source_urls':['https://opencitations.net/','https://download.opencitations.net/']+[a.get('figshare_url') or a.get('url_public_api') for a in articles],'license':'CC0-1.0; manifest metadata only','scope':'official Figshare article/file manifests only; no RDF/CSV dump payloads, citation edge rows, bibliographic record values, API/SPARQL harvest results, web page bodies, profiling/ranking outputs, or graph wiring','files':source_file_rows,'articles':articles,'archive_files':files,'dump_payloads_downloaded':False}
(out/'source-receipt.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True)+'\n', encoding='utf-8')
print(f'downloaded OpenCitations dump manifests: articles={len(articles)} archive_files={len(files)} total_size={sum((f.get("size") or 0) for f in files)} -> {out}')
PY
