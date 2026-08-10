#!/usr/bin/env bash
# NC-C-DEMO — operator durable KO corpus (seed + dialogue shards) → redb (+ optional FTS warm).
# Fast path (default): whole-corpus redb batch + parallel shard prepare.
# Skip FTS warm for dev: NL_CORPUS_DEMO_WARM_FTS=0
set -euo pipefail

REPO_ROOT="${PNIX_WORKSPACE_ROOT:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"
REDB_PATH="${1:-/tmp/pnix-nl-corpus-demo.redb}"
WARM_FTS="${NL_CORPUS_DEMO_WARM_FTS:-1}"
MANIFEST_PATH="${PNIXC_META_CORPUS_MANIFEST_PATH:-}"

cd "$REPO_ROOT"

if [[ ! -x target/debug/pnixc-meta ]]; then
  echo "building pnixc-meta..." >&2
  cargo build -p pnixc-meta
fi

mkdir -p "$(dirname "$REDB_PATH")"

ARGS=(--corpus-ingest-redb "$REDB_PATH")
if [[ -n "$MANIFEST_PATH" ]]; then
  ARGS+=(--corpus-manifest "$MANIFEST_PATH")
fi
if [[ "$WARM_FTS" == "1" ]]; then
  ARGS+=(--warm-fts)
fi

echo "pnixc-meta ${ARGS[*]}" >&2
target/debug/pnixc-meta "${ARGS[@]}"

echo "" >&2
echo "durable KO corpus ready: $REDB_PATH" >&2
echo "serve with:" >&2
echo "  export PNIX_WORKSPACE_ROOT=$REPO_ROOT" >&2
echo "  export PNIXC_META_CORPUS_REDB_PATH=$REDB_PATH" >&2
echo "  export PNIXC_META_CORPUS_PROJECTION_LOOKUP=1" >&2
echo "  export PNIXC_META_CORPUS_FTS_FALLBACK=1" >&2
echo "  export PNIXC_META_HTTP_PUNCHEETAH_CODE_CHAT=1" >&2
