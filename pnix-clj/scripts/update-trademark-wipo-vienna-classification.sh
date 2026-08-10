#!/usr/bin/env bash
# WIPO Vienna Classification publication XML snapshot.
# No explanatory notes/prose, actual images/logos, trademark application payloads, legal guidance, or graph wiring.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${VIENNA_DEST:-$ROOT/ingest/trademark/wipo-vienna-classification}"
VERSION="${VIENNA_VERSION:-10}"
LANG="${VIENNA_LANG:-en}"
URL="${VIENNA_URL:-https://nivilo.wipo.int/vienna10/xml/en/full.xml}"
rm -rf "$DEST"
mkdir -p "$DEST/raw"
python3 - "$DEST" "$VERSION" "$LANG" "$URL" <<'PY'
import datetime as dt, hashlib, json, pathlib, sys, urllib.request
out=pathlib.Path(sys.argv[1]); version=sys.argv[2]; lang=sys.argv[3]; url=sys.argv[4]
req=urllib.request.Request(url, headers={'User-Agent':'Mozilla/5.0 pnix-ingest-vienna/0.1'})
raw=urllib.request.urlopen(req, timeout=60).read()
(out/'raw'/'full.xml').write_bytes(raw)
receipt={'schema':'pnix.ingest.source_receipt.v1','source':'WIPO Vienna Classification official publication XML','version':version,'lang':lang,'retrieved_at':dt.datetime.now(dt.timezone.utc).isoformat(),'source_urls':['https://www.wipo.int/en/web/classification-vienna','https://www.wipo.int/classifications/vienna/en/ITsupport/Version20260101/index.html','https://nivilo.wipo.int/vienna.htm',url],'license':'WIPO official downloadable classification data / WIPO attribution family; category/division/section structure only','scope':'official VCL publication XML structure only; explanatory notes/prose, actual images/logos, trademark application records, legal guidance, linked payloads, and graph wiring excluded','files':[{'source_path':'raw/full.xml','relative_path':'raw/full.xml','url':url,'sha256':hashlib.sha256(raw).hexdigest(),'size_bytes':len(raw),'role':'wipo_vienna_full_xml'}],'explanatory_notes_downloaded':False,'image_logo_payloads_downloaded':False,'trademark_application_payloads_downloaded':False}
(out/'source-receipt.json').write_text(json.dumps(receipt, ensure_ascii=False, indent=2, sort_keys=True)+'\n', encoding='utf-8')
print(f'downloaded WIPO Vienna XML: version={version} lang={lang} bytes={len(raw)} -> {out}')
PY
