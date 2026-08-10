#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="${PNIX_WORKSPACE_ROOT:-$(git -C "$SCRIPT_DIR/.." rev-parse --show-toplevel 2>/dev/null || pwd)}"

FACTS_ONLY="${PNIX_DICTIONARY_FACTS_ONLY:-1}"
REDB_PATH="${PNIXC_META_REDB_PATH:-/tmp/pnix-nl-corpus-demo.redb}"

for arg in "$@"; do
  case "$arg" in
    --facts-only)
      FACTS_ONLY=1
      ;;
    --full)
      FACTS_ONLY=0
      ;;
    --redb=*)
      PNIXC_META_REDB_PATH="${arg#*=}"
      REDB_PATH="$PNIXC_META_REDB_PATH"
      ;;
    --help|-h)
      echo "usage: $(basename "$0") [--facts-only|--full] [--redb=PATH]" >&2
      echo "  --facts-only: definitions omitted, facts-only ingest mode" >&2
      echo "  --full:       keep definition text (facts-only off)" >&2
      echo "  --redb=PATH:  target redb path" >&2
      echo "  examples:" >&2
      echo "    $(basename "$0") --facts-only --redb=/tmp/test.redb" >&2
      echo "    $(basename "$0") --full --redb=./state/redb/demo.redb" >&2
      exit 0
      ;;
    *)
      echo "warning: ignoring unknown arg: $arg" >&2
      ;;
  esac
done

export PNIXC_META_REDB_PATH="$REDB_PATH"

echo "=== dual dictionary ingest 시작 ==="
echo "  Woorimalsaem: ingest/dictionary/woorimalsaem"
echo "  Stdict:      ingest/dictionary/stdict"
echo "  Redb:        $REDB_PATH"
echo "  Facts-only:   $FACTS_ONLY"
echo

export PNIX_DICTIONARY_FACTS_ONLY="$FACTS_ONLY"

"$SCRIPT_DIR/dictionary-woorimalsaem-ingest.sh"

echo "=== 우리말샘 적재 완료, 표준국어대사전 적재 시작 ==="

"$SCRIPT_DIR/dictionary-stdict-ingest.sh"

echo "=== dual dictionary ingest 완료 ==="
