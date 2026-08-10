#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/agriculture/usda-grin-catalog"
mkdir -p "$OUT"
UA="pnix-ingest/1.0 (catalog metadata only; contact: local)"
SEARCH_URL="${USDA_GRIN_SEARCH_URL:-https://npgsweb.ars-grin.gov/gringlobal/search}"
TAXON_URL="${USDA_GRIN_TAXON_URL:-https://npgsweb.ars-grin.gov/gringlobal/taxon/taxonomysearch}"
DESC_URL="${USDA_GRIN_DESCRIPTORS_URL:-https://npgsweb.ars-grin.gov/gringlobal/descriptors}"
curl -fsSL -A "$UA" "$SEARCH_URL" -o "$OUT/search.html"
curl -fsSL -A "$UA" "$TAXON_URL" -o "$OUT/taxonomysearch.html"
curl -fsSL -A "$UA" "$DESC_URL" -o "$OUT/descriptors.html"
python3 - "$OUT" "$SEARCH_URL" "$TAXON_URL" "$DESC_URL" <<'PY'
import hashlib,json,pathlib,sys,datetime
out=pathlib.Path(sys.argv[1]); urls=sys.argv[2:]
files=[]
for p in sorted(out.iterdir()):
    if p.name=='source-manifest.json' or not p.is_file(): continue
    files.append({'path':p.name,'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'bytes':p.stat().st_size})
manifest={'schema':'pnix.source_manifest.v1','source_id':'usda-grin-catalog','retrieved_at_utc':datetime.datetime.now(datetime.UTC).replace(microsecond=0).isoformat().replace('+00:00','Z'),'source_urls':urls,'files':files,'policy':'GRIN-Global catalog/search page metadata only; accession/taxon/descriptor payload rows and breeding/GMO guidance excluded'}
(out/'source-manifest.json').write_text(json.dumps(manifest,indent=2,sort_keys=True),encoding='utf-8')
print(json.dumps({'ok':True,'files':len(files),'out':str(out)},ensure_ascii=False))
PY
