#!/usr/bin/env bash
# Copy Korean codec / NL mirror .px dependency closure from ~/pnix into pnix-clj stdlib.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PNIX="${PNIX_ROOT:-$HOME/pnix}"
LIB="$PNIX/stdlib/lib"
DEST="$ROOT/stdlib/lib"

if [[ ! -d "$LIB/nl" ]]; then
  echo "missing pnix stdlib: $LIB" >&2
  exit 1
fi

mkdir -p "$DEST"

resolve_import() {
  local base="$1" imp="$2"
  imp="${imp%;*}"
  imp="${imp#"${imp%%[![:space:]]*}"}"
  imp="${imp%"${imp##*[![:space:]]}"}"
  if [[ "$imp" == /Users/* ]] || [[ "$imp" == "$PNIX"* ]]; then
    echo "$imp"
  elif [[ "$imp" == ./* ]]; then
    echo "$(cd "$(dirname "$base")" && pwd)/${imp#./}"
  elif [[ "$imp" == ../* ]]; then
    echo "$(cd "$(dirname "$base")" && cd "$(dirname "$imp")" 2>/dev/null && pwd)/$(basename "$imp")"
  else
    echo "$(cd "$(dirname "$base")" && pwd)/$imp"
  fi
}

queue=("$LIB/nl/korean-nl-mirror.px")
seen="$ROOT/.korean-codec-copy.seen"
: > "$seen"

while ((${#queue[@]})); do
  src="${queue[0]}"
  queue=("${queue[@]:1}")
  [[ -f "$src" ]] || continue
  grep -qxF "$src" "$seen" 2>/dev/null && continue
  echo "$src" >> "$seen"

  rel="${src#$LIB/}"
  dst="$DEST/$rel"
  mkdir -p "$(dirname "$dst")"
  cp "$src" "$dst"

  while IFS= read -r line; do
    imp="$(sed -n 's/.*import \([^;]*\);.*/\1/p' <<<"$line" | head -1)"
    [[ -n "$imp" ]] || continue
    resolved="$(resolve_import "$src" "$imp" 2>/dev/null || true)"
    [[ -f "$resolved" ]] || continue
    [[ "$resolved" == "$LIB"* ]] || continue
    queue+=("$resolved")
  done < <(grep -E 'import ' "$src" || true)
done

# Rewrite absolute pnix paths → relative within stdlib/lib
find "$DEST" -name '*.px' -print0 | while IFS= read -r -d '' f; do
  sed -i '' \
    -e "s|import ${PNIX}/stdlib/lib/|import |g" \
    -e "s|import /Users/[^/]*/pnix/stdlib/lib/|import |g" \
    "$f"
done

# Fix nl-layer imports that lost prefix (coding/foo → ../coding/foo from nl/)
for f in "$DEST"/nl/*.px; do
  [[ -f "$f" ]] || continue
  sed -i '' \
    -e 's|import nl/|import ./|g' \
    -e 's|import coding/|import ../coding/|g' \
    -e 's|import knowledge/|import ../knowledge/|g' \
    -e 's|import math/|import ../math/|g' \
    -e 's|import agent/|import ../agent/|g' \
    -e 's|import lisp/|import ../lisp/|g' \
    -e 's|import meta/|import ../meta/|g' \
    -e 's|import corpus/|import ../corpus/|g' \
    "$f"
done

find "$DEST"/nl -name '*.px' -exec sed -i '' 's|import nl/|import ./|g' {} \;

echo "korean codec closure: $(wc -l < "$seen" | tr -d ' ') px files"
rm -f "$seen"
