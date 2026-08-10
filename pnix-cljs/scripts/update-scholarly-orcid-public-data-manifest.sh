#!/usr/bin/env bash
# ORCID Public Data File Figshare article manifest snapshot.
# Uses official Figshare article metadata only. Does not download record tarballs and does not call ORCID Public API.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${ORCID_PUBLIC_DATA_DEST:-$ROOT/ingest/scholarly/orcid-public-data-manifest}"
ARTICLE_ID="${ORCID_FIGSHARE_ARTICLE_ID:-30375589}"
API_URL="${ORCID_FIGSHARE_API_URL:-https://api.figshare.com/v2/articles/$ARTICLE_ID}"
rm -rf "$DEST"
mkdir -p "$DEST/raw"
python3 - "$DEST" "$ARTICLE_ID" "$API_URL" <<'PY'
import datetime as dt, hashlib, json, pathlib, sys, urllib.request
out=pathlib.Path(sys.argv[1]); article_id=sys.argv[2]; api_url=sys.argv[3]
with urllib.request.urlopen(api_url, timeout=60) as r:
    raw=r.read()
article=json.loads(raw.decode('utf-8'))
(out/'raw'/'article.json').write_bytes(raw)
files=[]
for f in article.get('files') or []:
    files.append({
        'id': f.get('id'),
        'name': f.get('name'),
        'size': f.get('size'),
        'download_url': f.get('download_url'),
        'computed_md5': f.get('computed_md5'),
        'supplied_md5': f.get('supplied_md5')
    })
receipt={
    'schema':'pnix.ingest.source_receipt.v1',
    'source':'ORCID Public Data File Figshare article metadata',
    'article_id':article_id,
    'api_url':api_url,
    'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),
    'source_urls':['https://info.orcid.org/what-is-orcid/services/annual-data-files/','https://info.orcid.org/public-data-file-use-policy/',article.get('url_public_api') or api_url, article.get('figshare_url')],
    'license':'CC0-1.0; manifest metadata only',
    'scope':'official Figshare article/file manifest only; no ORCID API harvest, individual records, summaries, activities, profile values, emails, linked payloads, profiling, or graph wiring',
    'article':{
        'title': article.get('title'),
        'doi': article.get('doi'),
        'version': article.get('version'),
        'published_date': article.get('published_date'),
        'modified_date': article.get('modified_date'),
        'defined_type_name': article.get('defined_type_name'),
        'license': article.get('license'),
        'file_count': len(files),
        'total_file_size_bytes': sum((f.get('size') or 0) for f in files)
    },
    'files':[{'source_path':'raw/article.json','relative_path':'raw/article.json','url':api_url,'sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw),'role':'figshare_article_metadata_json'}],
    'archive_files':files,
    'tarball_payloads_downloaded':False
}
(out/'source-receipt.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True)+'\n', encoding='utf-8')
print(f'downloaded ORCID public data manifest: article_id={article_id} files={len(files)} total_size={receipt["article"]["total_file_size_bytes"]} -> {out}')
PY
