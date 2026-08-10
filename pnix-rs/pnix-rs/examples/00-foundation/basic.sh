#!/usr/bin/env bash
set -euo pipefail

PNIX_RS=${PNIX_RS:-pnix-rs}

echo "== basic value =="
"$PNIX_RS" px-eval -c 'let x = 20; in x + 22'

echo "== recursive attrset =="
"$PNIX_RS" px-eval -c 'rec { answer = base + 2; base = 40; }.answer'

echo "== guest held-shaped attrset remains a value =="
"$PNIX_RS" px-eval -c '{ status = "held"; value = 42; }'
