#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/registry/iana-special-purpose-addresses"
mkdir -p "$OUT"
URL4="https://www.iana.org/assignments/iana-ipv4-special-registry/iana-ipv4-special-registry.xml"
URL6="https://www.iana.org/assignments/iana-ipv6-special-registry/iana-ipv6-special-registry.xml"
curl -fsSL "$URL4" -o "$OUT/iana-ipv4-special-registry.xml"
curl -fsSL "$URL6" -o "$OUT/iana-ipv6-special-registry.xml"
python3 - <<'PY' "$OUT" "$URL4" "$URL6"
import hashlib, json, os, pathlib, sys
out=pathlib.Path(sys.argv[1]); urls=[sys.argv[2], sys.argv[3]]
files=[out/'iana-ipv4-special-registry.xml', out/'iana-ipv6-special-registry.xml']
items=[]
for p,u in zip(files, urls):
    b=p.read_bytes()
    items.append({
        'source_url': u,
        'source_path': str(p),
        'source_sha256': hashlib.sha256(b).hexdigest(),
        'source_bytes': len(b),
    })
manifest={
    'schema':'pnix.ingest.manifest.v1',
    'source_id':'iana-special-purpose-addresses',
    'project':'IANA Special-Purpose Address Registries',
    'snapshot_kind':'latest-content-addressed',
    'retrieved_at_utc': os.popen('date -u +%Y-%m-%dT%H:%M:%SZ').read().strip(),
    'license':'IANA any-purpose registry terms',
    'version_policy':'IANA registry has no release tags; update script fetches latest IPv4+IPv6 XML snapshots and redb key is content-addressed by generated pnix source hash.',
    'sources': items,
}
(out/'manifest.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2)+'\n', encoding='utf-8')
print(json.dumps(manifest, ensure_ascii=False, indent=2))
PY
