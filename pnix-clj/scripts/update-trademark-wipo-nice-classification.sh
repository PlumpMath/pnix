#!/usr/bin/env bash
# WIPO Nice Classification NCLPUB class-heading snapshot.
# No alphabetical term lists, explanatory notes/general remarks prose, legal guidance, trademark application payloads, or graph wiring.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${NICE_DEST:-$ROOT/ingest/trademark/wipo-nice-classification}"
VERSION="${NICE_VERSION:-20260101}"
LANG="${NICE_LANG:-en}"
URL="${NICE_URL:-https://nclpub.wipo.int/enfr/?explanatory_notes=hide&gors=&lang=$LANG&menulang=$LANG&notion=class_headings&version=$VERSION}"
rm -rf "$DEST"
mkdir -p "$DEST/raw"
python3 - "$DEST" "$VERSION" "$LANG" "$URL" <<'PY'
import datetime as dt, hashlib, json, pathlib, sys, urllib.request
out=pathlib.Path(sys.argv[1]); version=sys.argv[2]; lang=sys.argv[3]; url=sys.argv[4]
req=urllib.request.Request(url, headers={'User-Agent':'Mozilla/5.0 pnix-ingest-nice/0.1'})
raw=urllib.request.urlopen(req, timeout=60).read()
(out/'raw'/'class_headings.html').write_bytes(raw)
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'WIPO Nice Classification official NCLPUB class headings','version':version,'lang':lang,'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'source_urls':['https://www.wipo.int/en/web/classification-nice','https://www.wipo.int/classifications/nice/en/ITsupport/','https://www.wipo.int/classifications/nice/en/ITsupport/Version20260101/index.html',url],'license':'WIPO official downloadable classification data / WIPO attribution family; class headings only','scope':'official NCLPUB class headings only; no alphabetical terms, explanatory notes, general remarks, modifications, PDFs/Word/Excel payloads, trademark application records, legal guidance, linked payloads, or graph wiring','files':[{'source_path':'raw/class_headings.html','relative_path':'raw/class_headings.html','url':url,'sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw),'role':'wipo_nice_class_headings_html'}],'alphabetical_terms_downloaded':False,'explanatory_notes_downloaded':False,'trademark_application_payloads_downloaded':False}
(out/'source-receipt.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True)+'\n', encoding='utf-8')
print(f'downloaded WIPO Nice class headings: version={version} lang={lang} bytes={len(raw)} -> {out}')
PY
