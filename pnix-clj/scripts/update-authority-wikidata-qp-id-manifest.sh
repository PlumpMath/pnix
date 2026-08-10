#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${WIKIDATA_DEST:-$ROOT/ingest/authority/wikidata-qp-id-manifest}"
API="${WIKIDATA_API:-https://www.wikidata.org/w/api.php}"
IDS_CSV="${WIKIDATA_ENTITY_IDS:-Q5,Q42,Q146,Q6256,Q43229,P31,P279,P361,P17,P625}"
LANG="${WIKIDATA_LANG:-en}"
USER_AGENT="${WIKIDATA_USER_AGENT:-pnix-ingest/0.1 (bounded authority manifest; https://example.invalid/pnix)}"
mkdir -p "$DEST/raw"
ids_pipe="$(printf '%s' "$IDS_CSV" | tr ',' '|')"
site_url="$API?action=query&meta=siteinfo&siprop=general%7Cnamespaces%7Cnamespacealiases&format=json&formatversion=2"
entity_url="$API?action=wbgetentities&ids=$ids_pipe&props=info%7Clabels&languages=$LANG&format=json&formatversion=2"
printf 'Wikidata siteinfo 수집: %s\n' "$site_url" >&2
curl -fsSL -A "$USER_AGENT" "$site_url" -o "$DEST/raw/siteinfo.json"
printf 'Wikidata entity manifest 수집: ids=%s lang=%s (claims/sitelinks/descriptions/aliases 제외)\n' "$IDS_CSV" "$LANG" >&2
curl -fsSL -A "$USER_AGENT" "$entity_url" -o "$DEST/raw/entities.json"
python3 - <<'PY' "$DEST" "$API" "$IDS_CSV" "$LANG"
import hashlib, json, pathlib, sys, time
root=pathlib.Path(sys.argv[1]); api=sys.argv[2]; ids=sys.argv[3]; lang=sys.argv[4]
files=[]
for p in sorted((root/'raw').glob('*.json')):
    b=p.read_bytes(); files.append({'path': str(p.relative_to(root)), 'sha256': hashlib.sha256(b).hexdigest(), 'bytes': len(b)})
receipt={
  'schema':'authority.wikidata.qp_id_manifest.source_receipt.v1',
  'source':'Wikidata Action API',
  'api':api,
  'retrieved_at_unix':int(time.time()),
  'entity_ids':ids.split(','),
  'language':lang,
  'props':['info','labels'],
  'excluded_props':['claims','sitelinks','descriptions','aliases'],
  'license':'CC0-1.0 for Wikidata structured data',
  'files':files,
}
(root/'source-receipt.json').write_text(json.dumps(receipt,ensure_ascii=False,indent=2)+'\n')
PY
printf '완료: %s\n' "$DEST" >&2
