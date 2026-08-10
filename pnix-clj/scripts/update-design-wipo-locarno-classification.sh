#!/usr/bin/env bash
# WIPO Locarno Classification LOCPUB class/subclass heading snapshot.
# No alphabetical terms, explanatory notes/general remarks prose, legal guidance, design images, application payloads, or graph wiring.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${LOCARNO_DEST:-$ROOT/ingest/design/wipo-locarno-classification}"
VERSION="${LOCARNO_VERSION:-20270101}"
LANG="${LOCARNO_LANG:-en}"
URL="${LOCARNO_URL:-https://locpub.wipo.int/enfr/?explanatory_notes=hide&lang=$LANG&menulang=$LANG&notion=class_headings&subclasses=show&version=$VERSION}"
rm -rf "$DEST"
mkdir -p "$DEST/raw"
python3 - "$DEST" "$VERSION" "$LANG" "$URL" <<'PY'
import datetime as dt, hashlib, json, pathlib, sys, urllib.request
out=pathlib.Path(sys.argv[1]); version=sys.argv[2]; lang=sys.argv[3]; url=sys.argv[4]
req=urllib.request.Request(url, headers={'User-Agent':'Mozilla/5.0 pnix-ingest-locarno/0.1'})
raw=urllib.request.urlopen(req, timeout=90).read()
(out/'raw'/'class_subclass_headings.html').write_bytes(raw)
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'WIPO Locarno Classification official LOCPUB class/subclass headings','version':version,'lang':lang,'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'source_urls':['https://www.wipo.int/en/web/classification-locarno','https://www.wipo.int/classifications/locarno/en/ITsupport/','https://www.wipo.int/classifications/locarno/en/ITsupport/Version20250101/index.html',url],'license':'WIPO official downloadable classification data / WIPO attribution family; class/subclass headings only','scope':'official LOCPUB class/subclass headings only; no alphabetical terms, explanatory notes, general remarks, recommendations, modifications, PDFs/Word/Excel payloads, design application records/images, legal guidance, linked payloads, or graph wiring','files':[{'source_path':'raw/class_subclass_headings.html','relative_path':'raw/class_subclass_headings.html','url':url,'sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw),'role':'wipo_locarno_class_subclass_headings_html'}],'alphabetical_terms_downloaded':False,'explanatory_notes_downloaded':False,'design_image_payloads_downloaded':False,'design_application_payloads_downloaded':False}
(out/'source-receipt.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True)+'\n', encoding='utf-8')
print(f'downloaded WIPO Locarno class/subclass headings: version={version} lang={lang} bytes={len(raw)} -> {out}')
PY
