#!/usr/bin/env bash
# OPA/Rego metadata source updater.
# Downloads the current OPA binary so gen-code-opa-rego.sh can ask OPA for its own capabilities.
# Data stored later is metadata only: no policies, no decisions, no production authz data.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="$ROOT/ingest/code/opa"
TAG="${OPA_TAG:-latest}"
mkdir -p "$DEST"
API="https://api.github.com/repos/open-policy-agent/opa/releases"
if [[ "$TAG" == "latest" ]]; then
  REL_URL="$API/latest"
else
  REL_URL="$API/tags/$TAG"
fi
META="$DEST/release.json"
curl -fsSL "$REL_URL" -o "$META"
TAG="$(python3 - "$META" <<'PY'
import json,sys
print(json.load(open(sys.argv[1]))['tag_name'])
PY
)"
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64) WANT="opa_darwin_arm64_static" ;;
  Darwin-x86_64) WANT="opa_darwin_amd64" ;;
  Linux-x86_64) WANT="opa_linux_amd64_static" ;;
  Linux-aarch64|Linux-arm64) WANT="opa_linux_arm64_static" ;;
  *) echo "unsupported platform: $(uname -s)-$(uname -m)" >&2; exit 2 ;;
esac
python3 - "$META" "$WANT" <<'PY' > "$DEST/asset.env"
import json,sys
rel=json.load(open(sys.argv[1])); want=sys.argv[2]
assets={a['name']:a for a in rel.get('assets',[])}
if want not in assets:
    raise SystemExit(f"asset not found: {want}")
sha_name=want+'.sha256'
if sha_name not in assets:
    raise SystemExit(f"sha asset not found: {sha_name}")
print('ASSET_NAME='+want)
print('ASSET_URL='+assets[want]['browser_download_url'])
print('SHA_URL='+assets[sha_name]['browser_download_url'])
print('SIZE='+str(assets[want].get('size',0)))
PY
# shellcheck disable=SC1091
source "$DEST/asset.env"
BIN="$DEST/opa-$TAG-$ASSET_NAME"
TMP="$BIN.tmp"
SHA_TMP="$BIN.sha256.tmp"
curl -fL "$ASSET_URL" -o "$TMP"
curl -fsSL "$SHA_URL" -o "$SHA_TMP"
EXPECTED="$(awk '{print $1}' "$SHA_TMP")"
ACTUAL="$(shasum -a 256 "$TMP" | awk '{print $1}')"
if [[ "$EXPECTED" != "$ACTUAL" ]]; then
  echo "sha256 mismatch: expected=$EXPECTED actual=$ACTUAL" >&2
  exit 1
fi
mv "$TMP" "$BIN"
mv "$SHA_TMP" "$BIN.sha256"
chmod +x "$BIN"
ln -sf "$(basename "$BIN")" "$DEST/opa-current"
python3 - "$DEST/source-receipt.json" "$TAG" "$ASSET_NAME" "$ASSET_URL" "$ACTUAL" "$SIZE" <<'PY'
import json,sys,datetime
out,tag,asset,url,sha,size=sys.argv[1:]
json.dump({
  'schema':'pnix.ingest.source_receipt.v1',
  'source':'Open Policy Agent / Rego',
  'version':tag,
  'asset':asset,
  'url':url,
  'sha256':sha,
  'size_bytes':int(size),
  'retrieved_at':datetime.datetime.utcnow().replace(microsecond=0).isoformat()+'Z',
  'license':'Apache-2.0',
  'scope':'capabilities/builtin metadata only; no policies or production authorization data'
}, open(out,'w'), indent=2, ensure_ascii=False)
PY
"$DEST/opa-current" version || true
echo "updated OPA/Rego source: $TAG $ASSET_NAME $ACTUAL"
