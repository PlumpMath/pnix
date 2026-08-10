#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LIMIT="${MET_OBJECT_LIMIT:-60}"
OUT="$ROOT/ingest/culture/met-open-access"
mkdir -p "$OUT/objects"
receipt="$ROOT/corpus/culture/LICENSES/met-open-access.legal-provenance-receipt.json"
bash "$ROOT/scripts/require-legal-provenance-gate.sh" "met-open-access" "$receipt"
curl -L --fail --retry 3 --retry-delay 2 -o "$OUT/departments.json" "https://collectionapi.metmuseum.org/public/collection/v1/departments"
curl -L --fail --retry 3 --retry-delay 2 -o "$OUT/object_ids.json" "https://collectionapi.metmuseum.org/public/collection/v1/objects"
python3 - "$OUT" "$LIMIT" <<'PY'
import json, sys, time, urllib.request
from pathlib import Path
out=Path(sys.argv[1]); limit=int(sys.argv[2])
ids=json.load(open(out/'object_ids.json')).get('objectIDs',[])[:limit*6]
kept=0
for oid in ids:
    if kept>=limit: break
    p=out/'objects'/f'{oid}.json'
    if not p.exists():
        try:
            with urllib.request.urlopen(f'https://collectionapi.metmuseum.org/public/collection/v1/objects/{oid}', timeout=10) as r:
                data=r.read()
            p.write_bytes(data)
            time.sleep(0.02)
        except Exception:
            continue
    try: j=json.load(open(p))
    except Exception: continue
    if j.get('isPublicDomain') is True and int(j.get('objectEndDate') or 9999) <= 1925:
        kept += 1
print(f'met objects downloaded/filtered target={limit} kept_hint={kept}')
PY
( cd "$OUT" && find . -type f | sort | xargs shasum -a 256 > SHA256SUMS )
printf 'met-open-access updated: %s\n' "$OUT"
