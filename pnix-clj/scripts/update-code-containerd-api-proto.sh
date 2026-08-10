#!/usr/bin/env bash
set -euo pipefail

# Download/update the official containerd API protobuf snapshot.
# Default is a pinned release tag; override with CONTAINERD_API_PROTO_REF=<tag-or-branch>.
# This script only fetches source artifacts into gitignored ingest/; it does not wire graph/mirror code.

REF="${CONTAINERD_API_PROTO_REF:-v2.3.2}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/ingest/code/containerd-api-proto"
TMP="${TMPDIR:-/tmp}/pnix-containerd-api-proto-$$"
URL="https://github.com/containerd/containerd/archive/refs/tags/${REF}.tar.gz"
if [[ "$REF" == main || "$REF" == master || "$REF" == */* ]]; then
  URL="https://github.com/containerd/containerd/archive/refs/heads/${REF}.tar.gz"
fi

rm -rf "$TMP"
mkdir -p "$TMP" "$OUT/raw"
trap 'rm -rf "$TMP"' EXIT

printf 'containerd API protobuf update: ref=%s\n' "$REF" >&2
curl -L --fail --retry 3 --connect-timeout 20 -o "$TMP/containerd.tar.gz" "$URL"
SHA256="$(shasum -a 256 "$TMP/containerd.tar.gz" | awk '{print $1}')"
tar -xzf "$TMP/containerd.tar.gz" -C "$TMP"
SRC="$(find "$TMP" -maxdepth 1 -type d -name 'containerd-*' | head -1)"
if [[ -z "$SRC" || ! -d "$SRC/api" ]]; then
  echo "containerd api/ directory not found in archive" >&2
  exit 1
fi

rm -rf "$OUT/raw"
mkdir -p "$OUT/raw"
while IFS= read -r -d '' f; do
  rel="${f#$SRC/}"
  dest="$OUT/raw/${rel//\//__}"
  cp "$f" "$dest"
done < <(find "$SRC/api" -type f -name '*.proto' -print0 | sort -z)

cat > "$OUT/source-receipt.json" <<JSON
{
  "schema": "pnix.ingest.source_receipt.v1",
  "source_id": "containerd-api-proto",
  "source_name": "containerd API protobuf structural metadata",
  "ref": "${REF}",
  "archive_url": "${URL}",
  "archive_sha256": "${SHA256}",
  "license": "Apache-2.0",
  "retrieved_at_utc": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "raw_files": $(find "$OUT/raw" -type f -name '*.proto' | wc -l | tr -d ' '),
  "scope": "api/**/*.proto structural metadata only",
  "excluded": ["runtime daemon state", "actual containers/images/layers/snapshots/tasks", "credentials", "logs", "execution", "mirror/graph wiring"]
}
JSON

printf 'updated %s: proto_files=%s sha256=%s\n' "$OUT" "$(find "$OUT/raw" -type f -name '*.proto' | wc -l | tr -d ' ')" "$SHA256" >&2
