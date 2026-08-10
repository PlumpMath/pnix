#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/registry/iana-address-family-numbers"
URL="https://www.iana.org/assignments/address-family-numbers/address-family-numbers.xml"
mkdir -p "$OUT"
curl -fsSL "$URL" -o "$OUT/address-family-numbers.xml"
python3 - <<'PY' "$OUT" "$URL"
import hashlib, json, os, pathlib, sys
out=pathlib.Path(sys.argv[1]); url=sys.argv[2]; p=out/'address-family-numbers.xml'; b=p.read_bytes()
manifest={
  'schema':'pnix.ingest.manifest.v1',
  'source_id':'iana-address-family-numbers',
  'project':'IANA Address Family Numbers',
  'snapshot_kind':'latest-content-addressed',
  'retrieved_at_utc': os.popen('date -u +%Y-%m-%dT%H:%M:%SZ').read().strip(),
  'license':'IANA any-purpose registry terms',
  'version_policy':'IANA registry has no release tags; update script fetches latest XML and redb key is content-addressed by generated pnix source hash.',
  'source_url': url,
  'source_path': str(p),
  'source_sha256': hashlib.sha256(b).hexdigest(),
  'source_bytes': len(b),
}
(out/'manifest.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
print(json.dumps(manifest, ensure_ascii=False, indent=2))
PY
