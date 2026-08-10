#!/usr/bin/env bash
set -euo pipefail

PNIX_RS=${PNIX_RS:-pnix-rs}

echo "== rs-meta interprets and compiles the PNIX host mechanism =="
"$PNIX_RS" substrate-check

echo
echo "== PNIX self-interpreter agrees with native evaluation =="
"$PNIX_RS" tower-check

echo
echo "These are basic compiler/evaluator capabilities. Proof and service verdicts"
echo "remain separate consumers of their results."
