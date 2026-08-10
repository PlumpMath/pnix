#!/usr/bin/env bash
set -euo pipefail

PNIX_RS=${PNIX_RS:-pnix-rs}
SOURCE='{ answer = 20 + 22; flags = [ true false ]; }'

echo "== PNIX value -> Rust projection -> rs-meta substrate =="
"$PNIX_RS" rust-mirror -c "$SOURCE" \
  | grep -E 'px_value|program_sha256|status|loss_status'

echo
echo "The printed fields are observations. Structural HABI type authority is"
echo "carried by pnix.boundary-type.v1 nodes, not by these strings."
