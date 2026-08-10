#!/usr/bin/env bash
set -euo pipefail
ROOT="${PNIX_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
OUT_DIR="$ROOT/ingest/devops/kubernetes"
API="${K8S_RELEASE_API:-https://api.github.com/repos/kubernetes/kubernetes/releases/latest}"
mkdir -p "$OUT_DIR"
curl -fsSL "$API" -o "$OUT_DIR/release.json"
TAG="$(python3 - "$OUT_DIR/release.json" <<'PY'
import json, sys
print(json.load(open(sys.argv[1]))['tag_name'])
PY
)"
URL="${K8S_OPENAPI_URL:-https://raw.githubusercontent.com/kubernetes/kubernetes/$TAG/api/openapi-spec/swagger.json}"
OUT="$OUT_DIR/swagger-$TAG.json"
TMP="$OUT.tmp.$$"
echo "download: $URL"
curl -fL --retry 3 --retry-delay 2 "$URL" -o "$TMP"
mv "$TMP" "$OUT"
shasum -a 256 "$OUT" > "$OUT.sha256"
{
  echo "source_url=$URL"
  echo "tag=$TAG"
  echo "retrieved_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "sha256=$(cut -d' ' -f1 "$OUT.sha256")"
} > "$OUT.meta"
echo "wrote: $OUT"
cat "$OUT.sha256"
