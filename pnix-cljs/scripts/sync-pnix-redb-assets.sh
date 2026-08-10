#!/usr/bin/env bash
# Sync pnix redb DSL assets (stdlib store-plans, nl rules, ingest scripts) into pnix-clj.
# Raw dumps and .redb files are never copied — see ../.gitignore.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PNIX="${PNIX_ROOT:-$HOME/pnix}"

if [[ ! -d "$PNIX/stdlib/lib/nl" ]]; then
  echo "missing pnix checkout: set PNIX_ROOT or clone ~/pnix" >&2
  exit 1
fi

mkdir -p "$ROOT/stdlib/lib/nl" "$ROOT/stdlib/lib/corpus" "$ROOT/stdlib/lib/gate" \
         "$ROOT/stdlib/lib/meta" "$ROOT/corpus/dictionary" "$ROOT/docs/corpus" \
         "$ROOT/scripts" "$ROOT/ingest"

rsync -a --delete --exclude='*.generated.px' --exclude='*.generated.d/' \
  "$PNIX/stdlib/lib/nl/" "$ROOT/stdlib/lib/nl/"

find "$PNIX/stdlib/lib/corpus" -maxdepth 1 -type f \
  \( -name '*store-plan*.px' -o -name 'substrate-store-value-literal.px' \) \
  ! -name '*.generated.px' -exec cp {} "$ROOT/stdlib/lib/corpus/" \;

for f in substrate-store-execution-plan-shape-policy.px \
         pnixc-meta-store-config-shape-policy.px \
         store-index-direction-policy.px fact-store-index-policy.px; do
  cp "$PNIX/stdlib/lib/gate/$f" "$ROOT/stdlib/lib/gate/"
done

cp "$PNIX/stdlib/lib/meta/abstraction-status-store-plan"*.px "$ROOT/stdlib/lib/meta/" 2>/dev/null || true
rsync -a "$PNIX/corpus/dictionary/" "$ROOT/corpus/dictionary/"
cp "$PNIX/docs/corpus/redb-domain-recognition-gate.md" "$ROOT/docs/corpus/"
cp "$PNIX/docs/corpus/redb-domain-generic-matching-plan.md" "$ROOT/docs/corpus/"
cp "$PNIX/docs/corpus/allow-list-inventory.md" "$ROOT/docs/corpus/"
cp "$PNIX/ingest/README.md" "$ROOT/ingest/README.md"

find "$PNIX/scripts" -type f \( \
  -name '*dictionary*' -o -name '*redb*' -o -name '*store-plan*' -o \
  -name 'gen-*' -o -name 'update-*' -o -name 'load-*' -o \
  -name 'nl-corpus*' -o \
  -name 'require-legal*' -o -name 'check-redb*' -o -name 'check-license*' \
\) -exec cp {} "$ROOT/scripts/" \;

echo "synced from $PNIX"
echo "  nl px: $(find "$ROOT/stdlib/lib/nl" -name '*.px' | wc -l | tr -d ' ')"
echo "  corpus store-plan px: $(find "$ROOT/stdlib/lib/corpus" -name '*.px' | wc -l | tr -d ' ')"
echo "  scripts: $(find "$ROOT/scripts" -type f | wc -l | tr -d ' ')"
